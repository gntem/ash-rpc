//! Security configuration

use std::time::Duration;

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum number of concurrent connections (0 = unlimited)
    pub max_connections: usize,
    /// Maximum request size in bytes (0 = unlimited)
    pub max_request_size: usize,
    /// Request timeout duration
    pub request_timeout: Duration,
    /// Connection idle timeout
    pub idle_timeout: Duration,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            max_request_size: 1024 * 1024, // 1 MB
            request_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_security_config() {
        let config = SecurityConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.max_request_size, 1024 * 1024);
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
    }
}
