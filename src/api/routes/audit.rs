//! # Audit routes
//!
//! Read-only endpoints over the audit log. Audit records are append-only,
//! so there is no `POST`, `PUT`, `PATCH`, or `DELETE` here. New audit
//! entries are produced by service-layer code, never by HTTP clients.
//!
//! ## Authorization
//!
//! The entire `/audit` subtree sits behind the auth middleware, and the
//! handler itself enforces an `admin`-only check. Audit data is the most
//! sensitive in the system — it reveals who did what to whom — so access
//! is deliberately restricted to administrators even though the route is
//! only reading.

use axum::routing::get;
use axum::Router;

use crate::api::handlers::audit_handler::list_audit_for_entity;
use crate::app::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(list_audit_for_entity))
}