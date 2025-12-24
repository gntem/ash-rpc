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
