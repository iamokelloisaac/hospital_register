//! # Service Layer
//!
//! Business logic. Services orchestrate domain types and repositories to
//! fulfil use cases. See the module docs in [`crate::lib`] for the
//! architectural rules.
//!
//! ## What lives here
//!
//! - [`audit_service`] — writes and reads audit log entries.
//! - [`auth_service`] — verifies credentials, issues and verifies JWTs.
//! - [`doctor_service`] — CRUD for doctors, with password hashing.
//! - [`patient_service`] — CRUD for patients, with audit trail.
//! - [`appointment_service`] — booking and status transitions, with
//!   conflict detection.
//! - [`rate_limit_service`] — Redis-backed counters for the API middleware.
//!
//! ## What does **not** live here
//!
//! SQL. If a service method starts to look like `SELECT ...`, it is a
//! signal that a repository method is missing.

pub mod appointment_service;
pub mod audit_service;
pub mod auth_service;
pub mod doctor_service;
pub mod patient_service;
pub mod rate_limit_service;