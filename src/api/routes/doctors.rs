//! # Doctor routes
//!
//! Route table for the doctor resource. Nested in the protected subtree in
//! [`crate::api::routes::v1`], behind the auth middleware.
//!
//! ## Path syntax
//!
//! Dynamic segments are `{id}` in Axum 0.8 (they were `:id` in 0.7 and
//! earlier). See [`crate::api::routes::patients`] for the full rationale.

use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::api::handlers::doctor_handler::{
    create_doctor, delete_doctor, get_doctor, list_doctors, update_doctor,
};
use crate::app::state::AppState;

/// Returns the doctor sub-router.
pub fn routes() -> Router<AppState> {
    Router::new()
        // `POST /doctors` (create) and `GET /doctors` (list).
        .route("/", post(create_doctor).get(list_doctors))
        // `GET /doctors/{id}` (fetch one).
        .route("/{id}", get(get_doctor))
        // `PATCH /doctors/{id}` (partial update).
        .route("/{id}", patch(update_doctor))
        // `DELETE /doctors/{id}` (soft delete).
        .route("/{id}", delete(delete_doctor))
}