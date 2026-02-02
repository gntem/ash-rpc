//! Optional error sanitization utilities
//!
//! This module provides a trait-based approach for implementing custom
//! error sanitization. Library users can implement the `Sanitizer` trait
//! to define their own sanitization logic.
//!
//! # Example
//! ```
//! use ash_rpc::sanitization::Sanitizer;
//! use ash_rpc::Error;
//!
//! struct MyCustomSanitizer;
//!
//! impl Sanitizer for MyCustomSanitizer {
//!     fn sanitize(&self, error: &Error) -> Error {
//!         // Your custom logic here
//!         Error::new(error.code(), "Sanitized message")
//!     }
//! }
//!
//! # let error = Error::new(-32000, "Test");
//! let sanitized = error.sanitized_with(|e| MyCustomSanitizer.sanitize(e));
//! ```

use crate::Error;

/// Trait for implementing custom error sanitization logic
///
/// Implement this trait to define how errors should be sanitized
/// before being sent to clients. This gives you full control over
/// what information is exposed.
pub trait Sanitizer {
    /// Transform an error into a sanitized version
    ///
    /// # Arguments
    /// * `error` - The original error to sanitize
    ///
    /// # Returns
    /// A new Error with sanitized content
    fn sanitize(&self, error: &Error) -> Error;
}

/// Trait for applying transformations to strings
///
/// Implement this to create custom pattern-based transformations
/// for error messages and data.
pub trait PatternTransform {
    /// Apply the transformation to a string
    fn apply(&self, input: &str) -> String;
}

/// Simple pattern replacement implementation
pub struct SimplePattern {
    /// Pattern to search for (case-sensitive)
    pub pattern: String,
    /// Replacement text
    pub replacement: String,
}

impl SimplePattern {
    pub fn new(pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            replacement: replacement.into(),
        }
    }
}

impl PatternTransform for SimplePattern {
    fn apply(&self, input: &str) -> String {
        input.replace(&self.pattern, &self.replacement)
    }
}

/// Case-insensitive pattern replacement
pub struct CaseInsensitivePattern {
    /// Pattern to search for (case-insensitive)
    pub pattern: String,
    /// Replacement text
    pub replacement: String,
}

impl CaseInsensitivePattern {
    pub fn new(pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            replacement: replacement.into(),
        }
    }
}

impl PatternTransform for CaseInsensitivePattern {
    fn apply(&self, input: &str) -> String {
        let pattern_lower = self.pattern.to_lowercase();
        let input_lower = input.to_lowercase();

        if let Some(pos) = input_lower.find(&pattern_lower) {
            let mut result = input.to_string();
            result.replace_range(pos..pos + self.pattern.len(), &self.replacement);

            // Recursively handle multiple occurrences
            if result.to_lowercase().contains(&pattern_lower) {
                return self.apply(&result);
            }
            result
        } else {
            input.to_string()
        }
    }
}

/// Compose multiple transformations
pub struct CompositeTransform {
    transforms: Vec<Box<dyn PatternTransform + Send + Sync>>,
}

impl CompositeTransform {
    pub fn new() -> Self {
        Self {
            transforms: Vec::new(),
        }
    }

    pub fn add_transform<T: PatternTransform + Send + Sync + 'static>(
        mut self,
        transform: T,
    ) -> Self {
        self.transforms.push(Box::new(transform));
        self
    }
}

impl Default for CompositeTransform {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternTransform for CompositeTransform {
    fn apply(&self, input: &str) -> String {
        self.transforms
            .iter()
            .fold(input.to_string(), |acc, transform| transform.apply(&acc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_pattern() {
        let pattern = SimplePattern::new("password", "[REDACTED]");
        let result = pattern.apply("The password is secret123");
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("password"));
    }

    #[test]
    fn test_case_insensitive_pattern() {
        let pattern = CaseInsensitivePattern::new("password", "[REDACTED]");
        let result = pattern.apply("The PASSWORD is secret123");
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_composite_transform() {
        let composite = CompositeTransform::new()
            .add_transform(SimplePattern::new("password", "[REDACTED]"))
            .add_transform(SimplePattern::new("token", "[REDACTED]"));

        let result = composite.apply("password is secret, token is abc123");
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("password"));
        assert!(!result.contains("token"));
    }

    #[test]
    fn test_simple_pattern_no_match() {
        let pattern = SimplePattern::new("password", "[REDACTED]");
        let result = pattern.apply("This text has no sensitive data");
        assert_eq!(result, "This text has no sensitive data");
    }

    #[test]
    fn test_simple_pattern_multiple_occurrences() {
        let pattern = SimplePattern::new("key", "[REDACTED]");
        let result = pattern.apply("key1, key2, key3");
        assert_eq!(result.matches("[REDACTED]").count(), 3);
    }

    #[test]
    fn test_case_insensitive_pattern_various_cases() {
        let pattern = CaseInsensitivePattern::new("secret", "[REDACTED]");

        let result1 = pattern.apply("SECRET value");
        assert!(result1.contains("[REDACTED]"));

        let result2 = pattern.apply("Secret value");
        assert!(result2.contains("[REDACTED]"));

        let result3 = pattern.apply("secret value");
        assert!(result3.contains("[REDACTED]"));
    }

    #[test]
    fn test_case_insensitive_pattern_no_match() {
        let pattern = CaseInsensitivePattern::new("password", "[REDACTED]");
        let result = pattern.apply("No sensitive data here");
        assert_eq!(result, "No sensitive data here");
    }

    #[test]
    fn test_case_insensitive_pattern_multiple_occurrences() {
        let pattern = CaseInsensitivePattern::new("pass", "[REDACTED]");
        let result = pattern.apply("pass PASS Pass");
        // Should handle recursive replacements
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_composite_transform_empty() {
        let composite = CompositeTransform::new();
        let result = composite.apply("test string");
        assert_eq!(result, "test string");
    }

    #[test]
    fn test_composite_transform_default() {
        let composite = CompositeTransform::default();
        let result = composite.apply("test");
        assert_eq!(result, "test");
    }

    #[test]
    fn test_composite_transform_single() {
        let composite = CompositeTransform::new().add_transform(SimplePattern::new("test", "TEST"));

        let result = composite.apply("test string");
        assert_eq!(result, "TEST string");
    }

    #[test]
    fn test_composite_transform_chained() {
        let composite = CompositeTransform::new()
            .add_transform(SimplePattern::new("a", "b"))
            .add_transform(SimplePattern::new("b", "c"))
            .add_transform(SimplePattern::new("c", "d"));

        let result = composite.apply("a");
        assert_eq!(result, "d");
    }

    #[test]
    fn test_pattern_transform_trait() {
        let pattern: Box<dyn PatternTransform> = Box::new(SimplePattern::new("test", "replaced"));
        let result = pattern.apply("test value");
        assert_eq!(result, "replaced value");
    }
}
