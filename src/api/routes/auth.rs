//! # Auth routes
//!
//! Public authentication endpoints.
//!
//! ## Why login has its own limiter
//!
//! The global limit is loose (100 req/s per IP) — it exists to stop
//! floods, not to slow down credential stuffing. Login needs a much
//! tighter budget: 5 attempts per minute per IP. Rather than tightening
//! the global bucket, we attach a second, more restrictive limiter to the
//! login route alone.
//!
//! This layering means:
//!
//! - A failed login counts against **both** the global bucket and the
//!   login bucket. That is correct: both policies should see the attempt.
//! - Adding `/auth/logout` later gives us a place to choose a different
//!   policy (or none) without reworking the global layer.

use axum::middleware;
use axum::routing::post;
use axum::Router;

use crate::api::handlers::auth_handler::login;
use crate::api::middleware::rate_limit::login_rate_limit;
use crate::app::state::AppState;

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new().route(
        "/login",
        post(login).route_layer(middleware::from_fn_with_state(
            state,
            login_rate_limit,
        )),
    )
}