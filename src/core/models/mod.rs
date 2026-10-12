//! # Domain models
//!
//! Plain Rust structs and enums describing the entities the application
//! manages. These types are shared between the API layer (for serialization)
//! and the database layer (for query results).
//!
//! ## Conventions
//!
//! - Every model derives `Debug` and `Clone`.
//! - Models that cross the HTTP boundary additionally derive `Serialize`
//!   and/or `Deserialize`.
//! - Enums that are stored in Postgres implement `sqlx::Type` and are mapped
//!   to `TEXT` columns with a `CHECK` constraint. This makes migrations
//!   trivial compared to native Postgres `ENUM` types.

pub mod appointment;
pub mod audit_log;
pub mod doctor;
pub mod patient;