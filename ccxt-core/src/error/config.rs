//! Configuration validation error types.
//!
//! This module provides error types for configuration validation, allowing
//! developers to catch invalid configurations early with clear error messages.
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::error::{ConfigValidationError, ValidationResult};
//!
//! fn validate_max_retries(value: u32) -> Result<ValidationResult, ConfigValidationError> {
//!     if value > 10 {
//!         return Err(ConfigValidationError::ValueTooHigh {
//!             field: "max_retries",
//!             value: value.to_string(),
//!             max: "10".to_string(),
//!         });
//!     }
//!     Ok(ValidationResult::new())
//! }
//! ```

use std::fmt;
use thiserror::Error;

/// Configuration validation error types.
///
/// This enum represents different types of validation failures that can occur
/// when validating configuration parameters. Each variant includes the field
/// name and relevant values for debugging.
///
/// # Example
///
/// ```rust
/// use ccxt_core::error::ConfigValidationError;
///
/// let err = ConfigValidationError::ValueTooHigh {
///     field: "max_retries",
///     value: "15".to_string(),
///     max: "10".to_string(),
/// };
/// assert!(err.to_string().contains("max_retries"));
/// assert!(err.to_string().contains("15"));
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigValidationError {
    /// Field value exceeds the maximum allowed value.
    #[error("Field '{field}' value {value} exceeds maximum {max}")]
    ValueTooHigh {
        /// The name of the configuration field
        field: &'static str,
        /// The actual value that was provided
        value: String,
        /// The maximum allowed value
        max: String,
    },

    /// Field value is below the minimum allowed value.
    #[error("Field '{field}' value {value} is below minimum {min}")]
    ValueTooLow {
        /// The name of the configuration field
        field: &'static str,
        /// The actual value that was provided
        value: String,
        /// The minimum allowed value
        min: String,
    },

    /// Field value is invalid for reasons other than range.
    #[error("Field '{field}' has invalid value: {reason}")]
    ValueInvalid {
        /// The name of the configuration field
        field: &'static str,
        /// The reason why the value is invalid
        reason: String,
    },

    /// Required field is missing.
    #[error("Required field '{field}' is missing")]
    ValueMissing {
        /// The name of the missing configuration field
        field: &'static str,
    },
}

impl ConfigValidationError {
    /// Returns the field name associated with this error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ConfigValidationError;
    ///
    /// let err = ConfigValidationError::ValueTooHigh {
    ///     field: "max_retries",
    ///     value: "15".to_string(),
    ///     max: "10".to_string(),
    /// };
    /// assert_eq!(err.field_name(), "max_retries");
    /// ```
    #[must_use]
    pub fn field_name(&self) -> &'static str {
        match self {
            ConfigValidationError::ValueTooHigh { field, .. }
            | ConfigValidationError::ValueTooLow { field, .. }
            | ConfigValidationError::ValueInvalid { field, .. }
            | ConfigValidationError::ValueMissing { field } => field,
        }
    }

    /// Creates a new `ValueTooHigh` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ConfigValidationError;
    ///
    /// let err = ConfigValidationError::too_high("max_retries", 15, 10);
    /// assert!(err.to_string().contains("max_retries"));
    /// ```
    pub fn too_high<V: fmt::Display, M: fmt::Display>(
        field: &'static str,
        value: V,
        max: M,
    ) -> Self {
        ConfigValidationError::ValueTooHigh {
            field,
            value: value.to_string(),
            max: max.to_string(),
        }
    }

    /// Creates a new `ValueTooLow` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ConfigValidationError;
    ///
    /// let err = ConfigValidationError::too_low("base_delay_ms", 5, 10);
    /// assert!(err.to_string().contains("base_delay_ms"));
    /// ```
    pub fn too_low<V: fmt::Display, M: fmt::Display>(
        field: &'static str,
        value: V,
        min: M,
    ) -> Self {
        ConfigValidationError::ValueTooLow {
            field,
            value: value.to_string(),
            min: min.to_string(),
        }
    }

    /// Creates a new `ValueInvalid` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ConfigValidationError;
    ///
    /// let err = ConfigValidationError::invalid("capacity", "capacity cannot be zero");
    /// assert!(err.to_string().contains("capacity"));
    /// ```
    pub fn invalid(field: &'static str, reason: impl Into<String>) -> Self {
        ConfigValidationError::ValueInvalid {
            field,
            reason: reason.into(),
        }
    }

    /// Creates a new `ValueMissing` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ConfigValidationError;
    ///
    /// let err = ConfigValidationError::missing("api_key");
    /// assert!(err.to_string().contains("api_key"));
    /// ```
    pub fn missing(field: &'static str) -> Self {
        ConfigValidationError::ValueMissing { field }
    }
}

/// Result of a successful configuration validation.
///
/// This struct contains any warnings that were generated during validation.
/// Warnings indicate potential issues that don't prevent the configuration
/// from being used, but may cause suboptimal behavior.
///
/// # Example
///
/// ```rust
/// use ccxt_core::error::ValidationResult;
///
/// let mut result = ValidationResult::new();
/// result.add_warning("refill_period is very short, may cause high CPU usage");
/// assert!(!result.warnings.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationResult {
    /// Warnings generated during validation.
    ///
    /// These are non-fatal issues that the user should be aware of.
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Creates a new empty validation result.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ValidationResult;
    ///
    /// let result = ValidationResult::new();
    /// assert!(result.warnings.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    /// Creates a validation result with the given warnings.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ValidationResult;
    ///
    /// let result = ValidationResult::with_warnings(vec![
    ///     "Warning 1".to_string(),
    ///     "Warning 2".to_string(),
    /// ]);
    /// assert_eq!(result.warnings.len(), 2);
    /// ```
    #[must_use]
    pub fn with_warnings(warnings: Vec<String>) -> Self {
        Self { warnings }
    }

    /// Adds a warning to the validation result.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ValidationResult;
    ///
    /// let mut result = ValidationResult::new();
    /// result.add_warning("This is a warning");
    /// assert_eq!(result.warnings.len(), 1);
    /// ```
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Returns `true` if there are no warnings.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ValidationResult;
    ///
    /// let result = ValidationResult::new();
    /// assert!(result.is_ok());
    /// ```
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Returns `true` if there are any warnings.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ValidationResult;
    ///
    /// let mut result = ValidationResult::new();
    /// result.add_warning("Warning");
    /// assert!(result.has_warnings());
    /// ```
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Merges another validation result into this one.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::error::ValidationResult;
    ///
    /// let mut result1 = ValidationResult::new();
    /// result1.add_warning("Warning 1");
    ///
    /// let mut result2 = ValidationResult::new();
    /// result2.add_warning("Warning 2");
    ///
    /// result1.merge(result2);
    /// assert_eq!(result1.warnings.len(), 2);
    /// ```
    pub fn merge(&mut self, other: ValidationResult) {
        self.warnings.extend(other.warnings);
    }
}

#[cfg(test)]
#[allow(clippy::single_char_pattern)] // "5" is acceptable in tests
mod tests {
    use super::*;

    #[test]
    fn test_value_too_high_display() {
        let err = ConfigValidationError::ValueTooHigh {
            field: "max_retries",
            value: "15".to_string(),
            max: "10".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("max_retries"));
        assert!(msg.contains("15"));
        assert!(msg.contains("10"));
    }

    #[test]
    fn test_value_too_low_display() {
        let err = ConfigValidationError::ValueTooLow {
            field: "base_delay_ms",
            value: "5".to_string(),
            min: "10".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("base_delay_ms"));
        assert!(msg.contains("5"));
        assert!(msg.contains("10"));
    }

    #[test]
    fn test_value_invalid_display() {
        let err = ConfigValidationError::ValueInvalid {
            field: "capacity",
            reason: "capacity cannot be zero".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("capacity"));
        assert!(msg.contains("cannot be zero"));
    }

    #[test]
    fn test_value_missing_display() {
        let err = ConfigValidationError::ValueMissing { field: "api_key" };
        let msg = err.to_string();
        assert!(msg.contains("api_key"));
        assert!(msg.contains("missing"));
    }

    #[test]
    fn test_field_name() {
        let err1 = ConfigValidationError::too_high("max_retries", 15, 10);
        assert_eq!(err1.field_name(), "max_retries");

        let err2 = ConfigValidationError::too_low("base_delay_ms", 5, 10);
        assert_eq!(err2.field_name(), "base_delay_ms");

        let err3 = ConfigValidationError::invalid("capacity", "cannot be zero");
        assert_eq!(err3.field_name(), "capacity");

        let err4 = ConfigValidationError::missing("api_key");
        assert_eq!(err4.field_name(), "api_key");
    }

    #[test]
    fn test_helper_constructors() {
        let err1 = ConfigValidationError::too_high("field1", 100, 50);
        assert!(matches!(err1, ConfigValidationError::ValueTooHigh { .. }));

        let err2 = ConfigValidationError::too_low("field2", 5, 10);
        assert!(matches!(err2, ConfigValidationError::ValueTooLow { .. }));

        let err3 = ConfigValidationError::invalid("field3", "invalid reason");
        assert!(matches!(err3, ConfigValidationError::ValueInvalid { .. }));

        let err4 = ConfigValidationError::missing("field4");
        assert!(matches!(err4, ConfigValidationError::ValueMissing { .. }));
    }

    #[test]
    fn test_validation_result_new() {
        let result = ValidationResult::new();
        assert!(result.warnings.is_empty());
        assert!(result.is_ok());
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_validation_result_with_warnings() {
        let result =
            ValidationResult::with_warnings(vec!["Warning 1".to_string(), "Warning 2".to_string()]);
        assert_eq!(result.warnings.len(), 2);
        assert!(!result.is_ok());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::new();
        result.add_warning("Test warning");
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "Test warning");
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::new();
        result1.add_warning("Warning 1");

        let mut result2 = ValidationResult::new();
        result2.add_warning("Warning 2");
        result2.add_warning("Warning 3");

        result1.merge(result2);
        assert_eq!(result1.warnings.len(), 3);
    }

    #[test]
    fn test_config_validation_error_is_error() {
        // Verify that ConfigValidationError implements std::error::Error
        fn assert_error<E: std::error::Error>() {}
        assert_error::<ConfigValidationError>();
    }

    #[test]
    fn test_config_validation_error_clone() {
        let err = ConfigValidationError::too_high("field", 100, 50);
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }
}
