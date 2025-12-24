//! Trait-based logging abstraction for ash-rpc
//!
//! Provides a transport-agnostic logging interface with a default slog-rs implementation.

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
mod slog_impl {
    use super::*;
    use slog::{Drain, o};

    /// Slog-based logger implementation
    pub struct SlogLoggerImpl {
        logger: slog::Logger,
    }

    impl SlogLoggerImpl {
        /// Create a new slog logger with default configuration (terminal output)
        pub fn new() -> Self {
            let decorator = slog_term::TermDecorator::new().build();
            let drain = slog_term::FullFormat::new(decorator).build().fuse();
            let drain = slog_async::Async::new(drain).build().fuse();
            let logger = slog::Logger::root(drain, o!());
            
            Self { logger }
        }

        /// Create a new slog logger with JSON output
        pub fn new_json() -> Self {
            let drain = slog_json::Json::default(std::io::stdout()).fuse();
            let drain = slog_async::Async::new(drain).build().fuse();
            let logger = slog::Logger::root(drain, o!());
            
            Self { logger }
        }

        /// Create a logger from an existing slog Logger
        pub fn from_slog(logger: slog::Logger) -> Self {
            Self { logger }
        }

        /// Create a child logger with additional context
        pub fn child(&self, name: &str) -> Self {
            Self {
                logger: self.logger.new(o!("component" => name.to_string())),
            }
        }
    }

    impl Default for SlogLoggerImpl {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Logger for SlogLoggerImpl {
        fn debug(&self, message: &str, kvs: &[LogKv]) {
            let values: Vec<String> = kvs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            let kv_str = values.join(" ");
            slog::debug!(self.logger, "{} {}", message, kv_str);
        }

        fn info(&self, message: &str, kvs: &[LogKv]) {
            let values: Vec<String> = kvs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            let kv_str = values.join(" ");
            slog::info!(self.logger, "{} {}", message, kv_str);
        }

        fn warn(&self, message: &str, kvs: &[LogKv]) {
            let values: Vec<String> = kvs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            let kv_str = values.join(" ");
            slog::warn!(self.logger, "{} {}", message, kv_str);
        }

        fn error(&self, message: &str, kvs: &[LogKv]) {
            let values: Vec<String> = kvs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            let kv_str = values.join(" ");
            slog::error!(self.logger, "{} {}", message, kv_str);
        }
    }
}

#[cfg(feature = "logging")]
pub use slog_impl::SlogLoggerImpl;

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
    fn test_slog_logger() {
        let logger = SlogLoggerImpl::new();
        logger.info("test message", &[("method", &"test"), ("id", &123)]);
    }
}
