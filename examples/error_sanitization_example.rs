//! Example demonstrating error sanitization with user-defined callbacks
//!
//! This example shows how ash-rpc-core gives users full control over
//! error sanitization through callback functions and traits:
//! - Users decide what information to sanitize
//! - Custom transformation logic via callbacks
//! - Trait-based approach for reusable sanitization
//! - No built-in assumptions about what's "sensitive"
//!
//! Run with: cargo run --example error_sanitization_example

use ash_rpc_core::{
    Error, ErrorBuilder, error_codes,
    sanitization::{Sanitizer, PatternTransform, SimplePattern, CompositeTransform},
};

// Example 1: Custom Sanitizer implementation
struct ProductionSanitizer;

impl Sanitizer for ProductionSanitizer {
    fn sanitize(&self, error: &Error) -> Error {
        match error.code() {
            // Standard JSON-RPC errors pass through
            error_codes::PARSE_ERROR
            | error_codes::INVALID_REQUEST
            | error_codes::METHOD_NOT_FOUND
            | error_codes::INVALID_PARAMS => error.clone(),
            
            // Internal errors get generic message
            error_codes::INTERNAL_ERROR => {
                Error::new(error.code(), "Internal server error")
            }
            
            // Custom server errors also get generic message
            code if code >= -32099 && code <= -32000 => {
                Error::new(error.code(), "Server error")
            }
            
            // Other errors: keep message but remove data
            _ => Error {
                code: error.code(),
                message: error.message().to_string(),
                data: None,
            }
        }
    }
}

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("=== User-Controlled Error Sanitization ===\n");

    // Example 1: Using custom Sanitizer trait
    println!("1. Custom Sanitizer trait:");
    let internal_err = ErrorBuilder::new(
        error_codes::INTERNAL_ERROR,
        "Database connection failed: Connection refused at 192.168.1.100:5432"
    )
    .data(serde_json::json!({
        "connection_string": "postgresql://user:pass@localhost/db",
        "error_type": "ConnectionRefused"
    }))
    .build();
    
    let sanitizer = ProductionSanitizer;
    let sanitized = internal_err.sanitized_with(|e| sanitizer.sanitize(e));
    println!("   Original:  code={}, message={}", internal_err.code(), internal_err.message());
    println!("   Sanitized: code={}, message={}\n", sanitized.code(), sanitized.message());

    // Example 2: Simple callback for quick transforms
    println!("2. Simple callback transformation:");
    let custom_err = Error::new(-32001, "Failed to process user@example.com");
    
    let sanitized = custom_err.sanitized_with(|err| {
        Error::new(err.code(), "Failed to process user")
    });
    
    println!("   Original:  {}", custom_err.message());
    println!("   Sanitized: {}\n", sanitized.message());

    // Example 3: Using PatternTransform for text replacement
    println!("3. Pattern-based transformations:");
    let pattern_err = Error::new(
        -32002,
        "Authentication failed for password=secret123 and token=abc-xyz-123"
    );
    
    let pattern = SimplePattern::new("password=secret123", "password=[REDACTED]");
    let transformed_msg = pattern.apply(pattern_err.message());
    let sanitized = pattern_err.sanitized_with(|_| {
        Error::new(pattern_err.code(), transformed_msg)
    });
    
    println!("   Original:  {}", pattern_err.message());
    println!("   Sanitized: {}\n", sanitized.message());

    // Example 4: Composite transformations
    println!("4. Composite pattern transformations:");
    let multi_err = Error::new(
        -32003,
        "Error: password=secret, token=abc123, apikey=xyz789"
    );
    
    let composite = CompositeTransform::new()
        .add_transform(SimplePattern::new("password=secret", "password=[REDACTED]"))
        .add_transform(SimplePattern::new("token=abc123", "token=[REDACTED]"))
        .add_transform(SimplePattern::new("apikey=xyz789", "apikey=[REDACTED]"));
    
    let transformed = composite.apply(multi_err.message());
    let sanitized = multi_err.sanitized_with(|_| Error::new(multi_err.code(), transformed));
    
    println!("   Original:  {}", multi_err.message());
    println!("   Sanitized: {}\n", sanitized.message());

    // Example 5: Conditional sanitization based on error code
    println!("5. Conditional sanitization logic:");
    let errors = vec![
        Error::new(error_codes::INVALID_PARAMS, "Missing required field 'email'"),
        Error::new(error_codes::INTERNAL_ERROR, "Database query failed: syntax error"),
        Error::new(-32050, "Custom error with sensitive data"),
    ];
    
    for err in errors {
        let sanitized = err.sanitized_with(|e| {
            if e.code() == error_codes::INTERNAL_ERROR {
                Error::new(e.code(), "Internal server error")
            } else if e.code() >= -32099 && e.code() <= -32000 {
                Error::new(e.code(), "Server error")
            } else {
                e.clone()
            }
        });
        println!("   Code {}: {} -> {}", 
            err.code(), 
            err.message(), 
            sanitized.message()
        );
    }
    
    println!("\n=== Key Benefits ===");
    println!("✓ Users have full control over what gets sanitized");
    println!("✓ No assumptions about what's 'sensitive'");
    println!("✓ Flexible callback-based approach");
    println!("✓ Reusable sanitizers via Sanitizer trait");
    println!("✓ Composable transformations");
    println!("✓ Simple to understand and customize");
    println!("\nYou decide what information should be hidden!");
}
