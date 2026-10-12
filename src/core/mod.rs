//! # Core (Domain) Layer
//!
//! This module contains the **pure domain** of the application: the types and
//! traits that describe *what* the application is about, with no knowledge of
//! *how* it is delivered (HTTP) or *where* data is stored (Postgres).
//!
//! ## Rules
//!
//! 1. **No `axum` imports.** HTTP is a delivery mechanism, not a domain concern.
//! 2. **No `sqlx` imports.** Persistence is an infrastructure detail.
//! 3. **No `redis` imports.** Caching is an infrastructure detail.
//!
//! The only external crates `core` may use are general-purpose ones like
//! `serde`, `chrono`, `uuid`, and `thiserror`.
//!
//! ## Structure
//!
//! - [`errors`] — [`CoreError`](errors::CoreError), the domain-level error type.
//! - [`models`] — domain entities (Patient, Doctor, Appointment, …).
//! - [`traits`] — repository interfaces that the `db` layer implements.

pub mod errors;
pub mod models;
pub mod traits;