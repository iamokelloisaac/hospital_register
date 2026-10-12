//! # Shared utilities
//!
//! Cross-cutting concerns used by multiple layers. Nothing in this module
//! should depend on `api`, `db`, or `services`. It may depend on `core`.
//!
//! - [`crypto`] — password hashing and verification (Argon2).
//! - [`telemetry`] — tracing initialization and log formatting.

pub mod crypto;
pub mod telemetry;