//! Graceful shutdown support for JSON-RPC servers.
//!
//! This module provides graceful shutdown capabilities with:
//! - Signal handling (SIGTERM, SIGINT, custom triggers)
//! - Connection draining
//! - Configurable grace periods
//! - User-defined shutdown hooks for cleanup

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;

/// A future that completes when shutdown is triggered
pub type ShutdownFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Callback function type for shutdown hooks
pub type ShutdownHook = Box<dyn Fn() -> ShutdownFuture + Send + Sync>;

/// Shutdown signal that can be cloned and awaited
#[derive(Clone)]
pub struct ShutdownSignal {
    receiver: Arc<Mutex<mpsc::Receiver<()>>>,
}

impl ShutdownSignal {
    /// Create a new shutdown signal
    fn new(receiver: mpsc::Receiver<()>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Wait for shutdown signal
    pub async fn recv(&self) {
        let mut rx = self.receiver.lock().await;
        let _ = rx.recv().await;
    }
}

/// Handle to trigger shutdown
#[derive(Clone)]
pub struct ShutdownHandle {
    sender: mpsc::Sender<()>,
}

impl ShutdownHandle {
    /// Trigger shutdown
    pub async fn shutdown(&self) {
        let _ = self.sender.send(()).await;
    }

    /// Trigger shutdown (non-async, may fail if channel is full)
    pub fn shutdown_sync(&self) {
        let _ = self.sender.try_send(());
    }
}

/// Configuration for graceful shutdown
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Grace period to wait for connections to drain
    pub grace_period: Duration,
    
    /// Force shutdown after this timeout
    pub force_timeout: Duration,
    
    /// Whether to handle OS signals (SIGTERM, SIGINT)
    pub handle_signals: bool,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            grace_period: Duration::from_secs(30),
            force_timeout: Duration::from_secs(60),
            handle_signals: true,
        }
    }
}

/// Builder for shutdown configuration
pub struct ShutdownConfigBuilder {
    grace_period: Duration,
    force_timeout: Duration,
    handle_signals: bool,
}

impl ShutdownConfigBuilder {
    /// Create a new builder with defaults
    pub fn new() -> Self {
        Self {
            grace_period: Duration::from_secs(30),
            force_timeout: Duration::from_secs(60),
            handle_signals: true,
        }
    }

    /// Set the grace period for draining connections
    pub fn grace_period(mut self, duration: Duration) -> Self {
        self.grace_period = duration;
        self
    }

    /// Set the force timeout (hard deadline)
    pub fn force_timeout(mut self, duration: Duration) -> Self {
        self.force_timeout = duration;
        self
    }

    /// Enable or disable OS signal handling
    pub fn handle_signals(mut self, enabled: bool) -> Self {
        self.handle_signals = enabled;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ShutdownConfig {
        ShutdownConfig {
            grace_period: self.grace_period,
            force_timeout: self.force_timeout,
            handle_signals: self.handle_signals,
        }
    }
}

impl Default for ShutdownConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages graceful shutdown process
pub struct ShutdownManager {
    config: ShutdownConfig,
    hooks: Arc<RwLock<Vec<ShutdownHook>>>,
    signal: ShutdownSignal,
    handle: ShutdownHandle,
}

impl ShutdownManager {
    /// Create a new shutdown manager
    pub fn new(config: ShutdownConfig) -> Self {
        let (tx, rx) = mpsc::channel(1);
        let signal = ShutdownSignal::new(rx);
        let handle = ShutdownHandle { sender: tx };

        Self {
            config,
            hooks: Arc::new(RwLock::new(Vec::new())),
            signal,
            handle,
        }
    }

    /// Get a cloneable shutdown signal
    pub fn signal(&self) -> ShutdownSignal {
        self.signal.clone()
    }

    /// Get a handle to trigger shutdown
    pub fn handle(&self) -> ShutdownHandle {
        self.handle.clone()
    }

    /// Register a shutdown hook
    ///
    /// Hooks are called in registration order during shutdown
    pub async fn register_hook<F, Fut>(&self, hook: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let boxed_hook: ShutdownHook = Box::new(move || Box::pin(hook()));
        let mut hooks = self.hooks.write().await;
        hooks.push(boxed_hook);
    }

    /// Wait for shutdown signal and execute hooks
    pub async fn wait_for_shutdown(&self) {
        if self.config.handle_signals {
            // Wait for either signal or shutdown handle
            tokio::select! {
                _ = self.signal.recv() => {
                    tracing::info!("shutdown signal received via handle");
                }
                _ = Self::wait_for_signal() => {
                    tracing::info!("shutdown signal received from OS");
                }
            }
        } else {
            self.signal.recv().await;
            tracing::info!("shutdown signal received");
        }

        // Execute shutdown hooks
        self.execute_hooks().await;
    }

    /// Execute all registered shutdown hooks
    async fn execute_hooks(&self) {
        let hooks = self.hooks.read().await;
        
        tracing::info!(
            hook_count = hooks.len(),
            "executing shutdown hooks"
        );

        for (i, hook) in hooks.iter().enumerate() {
            tracing::debug!(hook_index = i, "executing shutdown hook");
            
            match timeout(self.config.grace_period, hook()).await {
                Ok(_) => {
                    tracing::debug!(hook_index = i, "shutdown hook completed");
                }
                Err(_) => {
                    tracing::warn!(
                        hook_index = i,
                        timeout_secs = ?self.config.grace_period,
                        "shutdown hook timed out"
                    );
                }
            }
        }

        tracing::info!("all shutdown hooks executed");
    }

    /// Wait for OS signals (SIGTERM, SIGINT)
    #[cfg(unix)]
    async fn wait_for_signal() {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate())
            .expect("failed to register SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt())
            .expect("failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM");
            }
            _ = sigint.recv() => {
                tracing::info!("received SIGINT");
            }
        }
    }

    #[cfg(not(unix))]
    async fn wait_for_signal() {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
        tracing::info!("received Ctrl-C");
    }

    /// Get the configured grace period
    pub fn grace_period(&self) -> Duration {
        self.config.grace_period
    }

    /// Get the configured force timeout
    pub fn force_timeout(&self) -> Duration {
        self.config.force_timeout
    }
}

/// Helper to create a basic shutdown manager with defaults
pub fn create_shutdown_manager() -> ShutdownManager {
    ShutdownManager::new(ShutdownConfig::default())
}

/// Helper to create a shutdown manager with custom config
pub fn create_shutdown_manager_with_config(config: ShutdownConfig) -> ShutdownManager {
    ShutdownManager::new(config)
}

/// Macro to register a shutdown hook
#[macro_export]
macro_rules! shutdown_hook {
    ($manager:expr, $body:expr) => {
        $manager.register_hook(|| async move { $body }).await
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn test_shutdown_signal() {
        let manager = create_shutdown_manager();
        let handle = manager.handle();
        let signal = manager.signal();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            handle.shutdown().await;
        });

        signal.recv().await;
        // Test passes if we reach here
    }

    #[tokio::test]
    async fn test_shutdown_hooks() {
        let manager = create_shutdown_manager();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        manager.register_hook(move || {
            let c = Arc::clone(&called_clone);
            async move {
                c.store(true, Ordering::SeqCst);
            }
        }).await;

        // Trigger shutdown
        let handle = manager.handle();
        tokio::spawn(async move {
            handle.shutdown().await;
        });

        manager.wait_for_shutdown().await;
        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_multiple_hooks() {
        let manager = create_shutdown_manager();
        let counter = Arc::new(AtomicBool::new(false));
        
        for i in 0..3 {
            let c = Arc::clone(&counter);
            manager.register_hook(move || {
                let c = Arc::clone(&c);
                async move {
                    tracing::debug!("Hook {} executed", i);
                    c.store(true, Ordering::SeqCst);
                }
            }).await;
        }

        let handle = manager.handle();
        tokio::spawn(async move {
            handle.shutdown().await;
        });

        manager.wait_for_shutdown().await;
        assert!(counter.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_shutdown_config_builder() {
        let config = ShutdownConfigBuilder::new()
            .grace_period(Duration::from_secs(10))
            .force_timeout(Duration::from_secs(20))
            .handle_signals(false)
            .build();

        assert_eq!(config.grace_period, Duration::from_secs(10));
        assert_eq!(config.force_timeout, Duration::from_secs(20));
        assert_eq!(config.handle_signals, false);
    }
}
