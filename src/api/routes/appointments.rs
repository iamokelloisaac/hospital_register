//! # Appointment routes
//!
//! Route table for the appointment resource. Nested in the protected
//! subtree in [`crate::api::routes::v1`], behind the auth middleware.
//!
//! ## Path syntax
//!
//! Dynamic segments are `{id}` in Axum 0.8. The sub-resource status
//! endpoint is `/{id}/status` — a static final segment after a dynamic
//! one, which is fine: Axum matches segment by segment, and a literal
//! segment is distinguished from a capture by the braces on the capture.
//!
//! ## Why `/status` is a separate path
//!
//! Status transitions have domain rules (terminal states are immutable).
//! Giving them their own endpoint means the audit trail is clean: every
//! `PATCH /appointments/{id}/status` produces one kind of audit entry,
//! distinct from a hypothetical future `PATCH /appointments/{id}` for
//! editing notes.

use axum::routing::{get, patch, post};
use axum::Router;

use crate::api::handlers::appointment_handler::{
    create_appointment, get_appointment, list_appointments,
    update_appointment_status,
};
use crate::app::state::AppState;

/// Returns the appointment sub-router.
pub fn routes() -> Router<AppState> {
    Router::new()
        // `POST /appointments`  → book
        // `GET  /appointments`  → list (filtered by query params)
        .route(
            "/",
            post(create_appointment).get(list_appointments),
        )
        // `GET /appointments/{id}` → fetch one
        .route("/{id}", get(get_appointment))
        // `PATCH /appointments/{id}/status` → transition
        .route("/{id}/status", patch(update_appointment_status))
}