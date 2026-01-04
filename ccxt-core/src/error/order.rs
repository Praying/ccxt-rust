//! Order-related error types.

use thiserror::Error;

/// Errors related to order management operations.
///
/// This type covers all order lifecycle errors including creation,
/// cancellation, and modification failures.
///
/// # Example
///
/// ```rust
/// use ccxt_core::error::{Error, OrderError};
///
/// fn cancel_order(order_id: &str) -> Result<(), Error> {
///     // Simulate order not found
///     if order_id == "unknown" {
///         return Err(OrderError::CancellationFailed(
///             format!("Order {} not found", order_id)
///         ).into());
///     }
///     Ok(())
/// }
/// ```
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum OrderError {
    /// Order creation failed.
    #[error("Order creation failed: {0}")]
    CreationFailed(String),

    /// Order cancellation failed.
    #[error("Order cancellation failed: {0}")]
    CancellationFailed(String),

    /// Order modification failed.
    #[error("Order modification failed: {0}")]
    ModificationFailed(String),

    /// Invalid order parameters.
    #[error("Invalid order parameters: {0}")]
    InvalidParameters(String),
}
