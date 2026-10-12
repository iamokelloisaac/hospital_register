//! # Data Transfer Objects
//!
//! Shapes for HTTP request and response bodies.
//!
//! ## Why not reuse domain models?
//!
//! Domain models (`core::models::Patient`, `core::models::Doctor`, …) may
//! contain fields that must **never** cross the HTTP boundary:
//!
//! - `deleted_at` — an internal soft-delete marker. Leaking it would tell
//!   clients which rows we have "removed" and when, which is private audit
//!   information.
//! - `password_hash` — never serialized, always hashed at rest. Keeping it
//!   off the DTO makes that fact structural rather than a matter of
//!   remembering not to add it.
//!
//! DTOs are the explicit, reviewable surface of the API. A change to a DTO
//! is an API change. A change to a domain model is not.
//!
//! ## Naming convention
//!
//! - Requests: `Create<Entity>Request`, `Update<Entity>Request`,
//!   `<Action><Entity>Request`.
//! - Responses: `<Entity>Response`.
//! - Request DTOs implement [`validator::Validate`] so they can be used
//!   with our [`ValidatedJson`](crate::api::extractors::validated_json::ValidatedJson)
//!   extractor.

pub mod appointment_dto;

pub mod auth_dto;
pub mod doctor_dto;
pub mod patient_dto;
pub mod audit_dto;