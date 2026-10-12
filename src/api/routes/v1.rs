//! # API v1
//!
//! The `/api/v1` route subtree. Every business endpoint lives under here.
//!
//! ## Why `routes` takes `AppState`
//!
//! In Axum 0.8, stateless middleware is attached with
//! [`axum::middleware::from_fn`], which expects a function of shape
//! `(Request, Next) -> Response`. Middleware that needs access to shared
//! state must instead be attached with
//! [`axum::middleware::from_fn_with_state`], which takes an owned copy of
//! the state. That signature forces `routes()` to receive `AppState` from
//! its caller.
//!
//! The alternatives are worse:
//!
//! - Storing `AppState` in a `static` (or `OnceLock`) and reading it from
//!   the middleware would work, but it hides the dependency. A reader of
//!   `v1.rs` would not see that the subtree needs state.
//! - Applying auth inside every handler would be repetitive and easy to
//!   forget. Middleware applied at the router level makes the safe path
//!   the default path.
//!
//! So we thread the state down. Both clones here are `Arc` pointer copies,
//! not deep copies — the cost is negligible.
//!
//! ## Layer ordering
//!
//! In `tower`, layers are applied in **reverse** order of declaration:
//! the last `.layer()` call becomes the outermost layer and runs *first*
//! for each request. Here:
//!
//! 1. `global_rate_limit` is applied last → runs first. A flood is
//!    rejected without spending CPU on JWT verification.
//! 2. `require_auth` is applied inside the protected group → runs after
//!    the limiter, only on protected routes.
//!
//! `route_layer` (rather than plain `layer`) is used for both because it
//! only runs for **matched routes**. Unmatched paths get a plain `404`
//! without invoking the limiter or the auth check — which is what we
//! want: an unknown path should not receive a misleading `401`.

use axum::middleware;
use axum::Router;

use crate::api::middleware::auth::require_auth;
use crate::api::middleware::rate_limit::global_rate_limit;
use crate::api::routes::{appointments, audit, auth, doctors, patients};
use crate::app::state::AppState;

/// Builds the `/api/v1` router with the given state.
pub fn routes(state: AppState) -> Router<AppState> {
    // Protected subtree: patients, doctors, appointments, audit.
    // `/auth` is deliberately excluded — a client has no token before
    // login, so wrapping it would deadlock the auth flow.
    let protected = Router::new()
        .nest("/patients", patients::routes())
        .nest("/doctors", doctors::routes())
        .nest("/appointments", appointments::routes())
        .nest("/audit", audit::routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ));

    Router::new()
        // Public login. Tight rate limit is applied inside `auth.rs`.
        .nest("/auth", auth::routes(state.clone()))
        // Everything behind auth.
        .merge(protected)
        // Global rate limit wraps the whole v1 subtree. Applied last so
        // it runs first for every v1 request.
        .route_layer(middleware::from_fn_with_state(
            state,
            global_rate_limit,
        ))
}