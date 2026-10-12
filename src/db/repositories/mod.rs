//! # Repositories
//!
//! Concrete implementations of the repository traits defined in
//! [`crate::core::traits`]. Every SQL query in the application lives here.
//!
//! ## Why all four are `pub mod`
//!
//! [`crate::app::state::AppState::new`] constructs one implementation of
//! each repository. That means the concrete structs (`PostgresPatientRepository`
//! and friends) must be reachable from outside this module. A private
//! `mod` would compile fine internally but would be invisible to `state.rs`.
//!
//! The convention `pub mod x;` re-exports the module, not everything
//! inside it. Each struct is still marked `pub struct`, and each method
//! is still `pub` or `pub(crate)` — nothing leaks that we did not
//! deliberately expose.

pub mod appointment_repo;
pub mod audit_repo;
pub mod doctor_repo;
pub mod patient_repo;