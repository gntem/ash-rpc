//! Macros for simplified observability setup

/// Simplifies observability stack initialization with a declarative syntax
///
/// # Examples
///
/// Basic setup with all features:
/// ```rust,ignore
/// use ash_rpc_contrib::observable_setup;
///
/// let observability = observable_setup! {
///     service_name: "my-service",
///     metrics_prefix: "my_app",
///     otlp_endpoint: "http://jaeger:4317",
/// };
/// ```
///
/// Minimal setup with just metrics:
/// ```rust,ignore
/// let observability = observable_setup! {
///     metrics_prefix: "my_app",
/// };
/// ```
///
/// With custom logger:
/// ```rust,ignore
/// let observability = observable_setup! {
///     service_name: "my-service",
///     metrics_prefix: "my_app",
///     otlp_endpoint: env!("JAEGER_ENDPOINT"),
///     log_level: "debug",
/// };
/// ```
#[macro_export]
macro_rules! observable_setup {
    // Full setup with all options
    (
        service_name: $service_name:expr,
        metrics_prefix: $metrics_prefix:expr,
        otlp_endpoint: $otlp_endpoint:expr
        $(, log_level: $log_level:expr)?
        $(,)?
    ) => {{
        #[allow(unused_imports)]
        use $crate::logging::{Logger, SlogLoggerImpl};

        // Setup logger
        let logger: ::std::sync::Arc<dyn Logger> = ::std::sync::Arc::new(SlogLoggerImpl::new());
        logger.info(
            "Initializing observability stack",
            &[
                ("service", &$service_name),
                ("metrics_prefix", &$metrics_prefix),
            ],
        );

        // Initialize OpenTelemetry tracer
        #[cfg(feature = "opentelemetry")]
        {
            use ::opentelemetry::global;
            use ::opentelemetry_otlp::{SpanExporter, WithExportConfig};
            use ::opentelemetry_sdk::Resource;
            use ::opentelemetry_sdk::trace::TracerProvider;

            logger.info(
                "Initializing OpenTelemetry tracer",
                &[("endpoint", &$otlp_endpoint)],
            );

            let exporter = SpanExporter::builder()
                .with_tonic()
                .with_endpoint($otlp_endpoint)
                .build()
                .expect("Failed to create OTLP exporter");

            let tracer_provider = TracerProvider::builder()
                .with_batch_exporter(exporter, ::opentelemetry_sdk::runtime::Tokio)
                .with_resource(Resource::new(vec![::opentelemetry::KeyValue::new(
                    "service.name",
                    $service_name,
                )]))
                .build();

            global::set_tracer_provider(tracer_provider);
            logger.info("OpenTelemetry tracer initialized", &[]);
        }

        // Create Prometheus metrics
        #[cfg(feature = "prometheus")]
        let metrics = {
            let m = ::std::sync::Arc::new(
                $crate::observability::prometheus::PrometheusMetrics::with_prefix($metrics_prefix)
                    .expect("Failed to create Prometheus metrics"),
            );

            // Register process metrics on Linux
            #[cfg(all(target_os = "linux", feature = "prometheus"))]
            {
                let process_collector =
                    ::prometheus::process_collector::ProcessCollector::for_self();
                m.registry()
                    .register(Box::new(process_collector))
                    .expect("Failed to register process collector");
                logger.info("Prometheus metrics initialized with process collector", &[]);
            }

            #[cfg(not(target_os = "linux"))]
            logger.info("Prometheus metrics initialized", &[]);

            m
        };

        #[cfg(not(feature = "prometheus"))]
        let metrics = ();

        $crate::observability::macros::ObservabilityStack {
            #[cfg(feature = "prometheus")]
            metrics,
            logger,
        }
    }};

    // Minimal setup - just metrics
    (
        metrics_prefix: $metrics_prefix:expr
        $(,)?
    ) => {{
        #[allow(unused_imports)]
        use $crate::logging::{Logger, SlogLoggerImpl};

        let logger: ::std::sync::Arc<dyn Logger> = ::std::sync::Arc::new(SlogLoggerImpl::new());
        logger.info(
            "Initializing observability stack",
            &[("metrics_prefix", &$metrics_prefix)],
        );

        #[cfg(feature = "prometheus")]
        let metrics = {
            let m = ::std::sync::Arc::new(
                $crate::observability::prometheus::PrometheusMetrics::with_prefix($metrics_prefix)
                    .expect("Failed to create Prometheus metrics"),
            );

            #[cfg(all(target_os = "linux", feature = "prometheus"))]
            {
                let process_collector =
                    ::prometheus::process_collector::ProcessCollector::for_self();
                m.registry()
                    .register(Box::new(process_collector))
                    .expect("Failed to register process collector");
                logger.info("Prometheus metrics initialized with process collector", &[]);
            }

            #[cfg(not(target_os = "linux"))]
            logger.info("Prometheus metrics initialized", &[]);

            m
        };

        #[cfg(not(feature = "prometheus"))]
        let metrics = ();

        $crate::observability::macros::ObservabilityStack {
            #[cfg(feature = "prometheus")]
            metrics,
            logger,
        }
    }};

    // With service name and metrics only (no tracing)
    (
        service_name: $service_name:expr,
        metrics_prefix: $metrics_prefix:expr
        $(,)?
    ) => {{
        #[allow(unused_imports)]
        use $crate::logging::{Logger, SlogLoggerImpl};

        let logger: ::std::sync::Arc<dyn Logger> = ::std::sync::Arc::new(SlogLoggerImpl::new());
        logger.info(
            "Initializing observability stack",
            &[
                ("service", &$service_name),
                ("metrics_prefix", &$metrics_prefix),
            ],
        );

        #[cfg(feature = "prometheus")]
        let metrics = {
            let m = ::std::sync::Arc::new(
                $crate::observability::prometheus::PrometheusMetrics::with_prefix($metrics_prefix)
                    .expect("Failed to create Prometheus metrics"),
            );

            #[cfg(all(target_os = "linux", feature = "prometheus"))]
            {
                let process_collector =
                    ::prometheus::process_collector::ProcessCollector::for_self();
                m.registry()
                    .register(Box::new(process_collector))
                    .expect("Failed to register process collector");
                logger.info("Prometheus metrics initialized with process collector", &[]);
            }

            #[cfg(not(target_os = "linux"))]
            logger.info("Prometheus metrics initialized", &[]);

            m
        };

        #[cfg(not(feature = "prometheus"))]
        let metrics = ();

        $crate::observability::macros::ObservabilityStack {
            #[cfg(feature = "prometheus")]
            metrics,
            logger,
        }
    }};
}

/// Container for observability components returned by observable_setup!
pub struct ObservabilityStack {
    #[cfg(feature = "prometheus")]
    pub metrics: ::std::sync::Arc<super::prometheus::PrometheusMetrics>,
    pub logger: ::std::sync::Arc<dyn crate::logging::Logger>,
}

impl ObservabilityStack {
    /// Get the metrics collector
    #[cfg(feature = "prometheus")]
    pub fn metrics(&self) -> ::std::sync::Arc<super::prometheus::PrometheusMetrics> {
        ::std::sync::Arc::clone(&self.metrics)
    }

    /// Get the logger
    pub fn logger(&self) -> ::std::sync::Arc<dyn crate::logging::Logger> {
        ::std::sync::Arc::clone(&self.logger)
    }
}
