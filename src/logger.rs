//! Logging trait for ash-rpc-core
//!
//! Simple logging abstraction for internal use.

use std::fmt::Display;

/// Key-value pair for structured logging
pub type LogKv<'a> = (&'a str, &'a dyn Display);

/// Logging trait for ash-rpc components
pub trait Logger: Send + Sync {
    /// Log a debug message
    fn debug(&self, message: &str, kvs: &[LogKv]);

    /// Log an info message
    fn info(&self, message: &str, kvs: &[LogKv]);

    /// Log a warning message
    fn warn(&self, message: &str, kvs: &[LogKv]);

    /// Log an error message
    fn error(&self, message: &str, kvs: &[LogKv]);
}

/// No-op logger that discards all messages (default)
#[derive(Debug, Clone, Copy)]
pub struct NoopLogger;

impl Logger for NoopLogger {
    fn debug(&self, _message: &str, _kvs: &[LogKv]) {}
    fn info(&self, _message: &str, _kvs: &[LogKv]) {}
    fn warn(&self, _message: &str, _kvs: &[LogKv]) {}
    fn error(&self, _message: &str, _kvs: &[LogKv]) {}
}

impl Default for NoopLogger {
    fn default() -> Self {
        Self
    }
}

/// Simple stdout logger for basic logging needs
#[derive(Debug, Clone, Copy)]
pub struct StdoutLogger;

impl Logger for StdoutLogger {
    fn debug(&self, message: &str, kvs: &[LogKv]) {
        print!("[DEBUG] {}", message);
        for (k, v) in kvs {
            print!(" {}={}", k, v);
        }
        println!();
    }

    fn info(&self, message: &str, kvs: &[LogKv]) {
        print!("[INFO] {}", message);
        for (k, v) in kvs {
            print!(" {}={}", k, v);
        }
        println!();
    }

    fn warn(&self, message: &str, kvs: &[LogKv]) {
        print!("[WARN] {}", message);
        for (k, v) in kvs {
            print!(" {}={}", k, v);
        }
        println!();
    }

    fn error(&self, message: &str, kvs: &[LogKv]) {
        eprint!("[ERROR] {}", message);
        for (k, v) in kvs {
            eprint!(" {}={}", k, v);
        }
        eprintln!();
    }
}

/// Tracing-based logger implementation using the `tracing` crate
#[derive(Debug, Clone, Copy)]
pub struct TracingLogger;

impl TracingLogger {
    /// Create a new tracing logger
    pub fn new() -> Self {
        Self
    }
}

impl Default for TracingLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger for TracingLogger {
    fn debug(&self, message: &str, kvs: &[LogKv]) {
        if kvs.is_empty() {
            tracing::debug!("{}", message);
        } else {
            let fields: Vec<String> = kvs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            tracing::debug!("{} {}", message, fields.join(" "));
        }
    }

    fn info(&self, message: &str, kvs: &[LogKv]) {
        if kvs.is_empty() {
            tracing::info!("{}", message);
        } else {
            let fields: Vec<String> = kvs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            tracing::info!("{} {}", message, fields.join(" "));
        }
    }

    fn warn(&self, message: &str, kvs: &[LogKv]) {
        if kvs.is_empty() {
            tracing::warn!("{}", message);
        } else {
            let fields: Vec<String> = kvs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            tracing::warn!("{} {}", message, fields.join(" "));
        }
    }

    fn error(&self, message: &str, kvs: &[LogKv]) {
        if kvs.is_empty() {
            tracing::error!("{}", message);
        } else {
            let fields: Vec<String> = kvs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            tracing::error!("{} {}", message, fields.join(" "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_logger_methods() {
        let logger = NoopLogger;

        // These should not panic and should do nothing
        logger.debug("debug message", &[]);
        logger.info("info message", &[]);
        logger.warn("warn message", &[]);
        logger.error("error message", &[]);
    }

    #[test]
    fn test_noop_logger_with_kvs() {
        let logger = NoopLogger;
        let num = 42;
        let text = "value";

        let kvs: &[LogKv] = &[("key1", &num), ("key2", &text)];

        // These should not panic and should do nothing
        logger.debug("debug with kvs", kvs);
        logger.info("info with kvs", kvs);
        logger.warn("warn with kvs", kvs);
        logger.error("error with kvs", kvs);
    }

    #[test]
    fn test_noop_logger_default() {
        let logger = NoopLogger::default();
        logger.info("test", &[]);
        // Should not panic
    }

    #[test]
    fn test_stdout_logger_no_kvs() {
        let logger = StdoutLogger;

        // These should output to stdout/stderr without panicking
        logger.debug("debug message", &[]);
        logger.info("info message", &[]);
        logger.warn("warn message", &[]);
        logger.error("error message", &[]);
    }

    #[test]
    fn test_stdout_logger_with_kvs() {
        let logger = StdoutLogger;
        let num = 42;
        let text = "test_value";

        let kvs: &[LogKv] = &[("count", &num), ("name", &text)];

        // These should output to stdout/stderr with key-value pairs
        logger.debug("debug with data", kvs);
        logger.info("info with data", kvs);
        logger.warn("warn with data", kvs);
        logger.error("error with data", kvs);
    }

    #[test]
    fn test_tracing_logger_new() {
        let logger = TracingLogger::new();

        // Basic test - should not panic
        logger.debug("test", &[]);
    }

    #[test]
    fn test_tracing_logger_default() {
        let logger = TracingLogger::default();

        // Basic test - should not panic
        logger.info("test", &[]);
    }

    #[test]
    fn test_tracing_logger_no_kvs() {
        let logger = TracingLogger;

        // These should use tracing without panicking
        logger.debug("debug message", &[]);
        logger.info("info message", &[]);
        logger.warn("warn message", &[]);
        logger.error("error message", &[]);
    }

    #[test]
    fn test_tracing_logger_with_kvs() {
        let logger = TracingLogger;
        let num = 100;
        let text = "data";

        let kvs: &[LogKv] = &[("metric", &num), ("label", &text)];

        // These should format with key-value pairs
        logger.debug("debug with kvs", kvs);
        logger.info("info with kvs", kvs);
        logger.warn("warn with kvs", kvs);
        logger.error("error with kvs", kvs);
    }

    #[test]
    fn test_logger_trait_object() {
        let loggers: Vec<Box<dyn Logger>> = vec![
            Box::new(NoopLogger),
            Box::new(StdoutLogger),
            Box::new(TracingLogger),
        ];

        for logger in loggers {
            logger.info("test message", &[]);
        }
    }

    #[test]
    fn test_logger_with_multiple_kvs() {
        let logger = StdoutLogger;
        let a = 1;
        let b = 2;
        let c = "three";

        let kvs: &[LogKv] = &[("a", &a), ("b", &b), ("c", &c)];
        logger.info("multiple kvs", kvs);
    }

    #[test]
    fn test_logger_empty_message() {
        let logger = NoopLogger;
        logger.info("", &[]);

        let logger = StdoutLogger;
        logger.debug("", &[]);
    }

    #[test]
    fn test_noop_logger_clone() {
        let logger1 = NoopLogger;
        let logger2 = logger1;

        logger1.info("from logger1", &[]);
        logger2.info("from logger2", &[]);
    }

    #[test]
    fn test_stdout_logger_clone() {
        let logger1 = StdoutLogger;
        let logger2 = logger1;

        logger1.info("from logger1", &[]);
        logger2.info("from logger2", &[]);
    }

    #[test]
    fn test_tracing_logger_clone() {
        let logger1 = TracingLogger;
        let logger2 = logger1;

        logger1.info("from logger1", &[]);
        logger2.info("from logger2", &[]);
    }
}
