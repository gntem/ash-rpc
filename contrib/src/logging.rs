//! Trait-based logging abstraction for ash-rpc
//!
//! Provides a transport-agnostic logging interface with a tracing backend.

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

/// No-op logger that discards all messages
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

#[cfg(feature = "logging")]
mod tracing_impl {
    use super::*;

    /// Tracing-based logger implementation
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
}

#[cfg(feature = "logging")]
pub use tracing_impl::TracingLogger;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_logger() {
        let logger = NoopLogger;
        logger.info("test", &[("key", &"value")]);
        logger.debug("debug", &[]);
        logger.warn("warn", &[("code", &42)]);
        logger.error("error", &[]);
    }

    #[cfg(feature = "logging")]
    #[test]
    fn test_tracing_logger() {
        let logger = TracingLogger::new();
        logger.info("test message", &[("method", &"test"), ("id", &123)]);
    }
}
