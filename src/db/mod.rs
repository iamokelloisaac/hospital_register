//! # Database Layer
//!
//! Everything that touches Postgres or Redis lives here.
//!
//! - [`pool`] — constructs and validates the `PgPool` and Redis
//!   `ConnectionManager`.
//! - [`repositories`] — concrete implementations of the traits defined in
//!   [`crate::core::traits`].
//!
//! ## Rule
//!
//! No module outside `db` should import `sqlx` or `redis` directly. This
//! keeps the rest of the application free to swap persistence strategies.

pub mod pool;
pub mod repositories;