//! # Patient routes
//!
//! Route table for the patient resource.
//!
//! ## Path syntax in Axum 0.8
//!
//! Dynamic path segments use **braces**, not colons:
//!
//! - Axum 0.7 and earlier: `/:id`
//! - Axum 0.8 and later:   `/{id}`
//!
//! The change aligns Axum with [RFC 6570][rfc6570] URI Templates, which is
//! also what OpenAPI 3.1 uses. It also frees the colon to mean a literal
//! colon in a path segment. If you see
//! `Path segments must not start with ':'`, this is why.
//!
//! [rfc6570]: https://datatracker.ietf.org/doc/html/rfc6570
//!
//! ## Where auth lives
//!
//! Auth is **not** applied here. This router is nested inside the
//! "protected" subtree in [`crate::api::routes::v1`], which wraps the
//! whole group with the auth middleware in one place. Adding a new route
//! here is therefore protected by default.

use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::api::handlers::patient_handler::{
    create_patient, delete_patient, get_patient, list_patients,
    update_patient,
};
use crate::app::state::AppState;

/// Returns the patient sub-router.
pub fn routes() -> Router<AppState> {
    Router::new()
        // `POST /patients` (create) and `GET /patients` (list).
        .route("/", post(create_patient).get(list_patients))
        // `GET /patients/{id}` (fetch one).
        .route("/{id}", get(get_patient))
        // `PATCH /patients/{id}` (partial update).
        .route("/{id}", patch(update_patient))
        // `DELETE /patients/{id}` (soft delete).
        .route("/{id}", delete(delete_patient))
}