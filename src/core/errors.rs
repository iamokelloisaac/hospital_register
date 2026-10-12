//! # Core errors
//!
//! [`CoreError`] is the single error type that the **domain and service**
//! layers use. It is deliberately HTTP-agnostic: the API layer is responsible
//! for mapping each variant to an appropriate HTTP status code.
//!
//! ## Why a dedicated error type?
//!
//! If services returned `sqlx::Error` directly, they would leak a database
//! concern into the domain layer. Instead, repository implementations
//! translate `sqlx::Error` into `CoreError` variants, and the API layer
//! translates `CoreError` into HTTP responses. Each layer speaks its own
//! vocabulary.

use thiserror::Error;

/// Domain-level errors.
///
/// Each variant carries a human-readable message. In a production system you
/// might also want to attach a machine-readable code or structured metadata;
/// we keep it simple for now.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Input failed validation (e.g., malformed NIN, invalid date range).
    #[error("validation failed: {0}")]
    Validation(String),

    /// A requested entity does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// The operation conflicts with existing state (e.g., duplicate NIN).
    #[error("conflict: {0}")]
    Conflict(String),

    /// The caller is not authenticated.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// The caller is authenticated but lacks permission.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// An unexpected internal error occurred.
    ///
    /// The message is for logging only; it must never be shown to clients.
    #[error("internal error: {0}")]
    Internal(String),
}