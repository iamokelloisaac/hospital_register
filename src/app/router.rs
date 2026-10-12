//! # Router construction
//!
//! Builds the top-level Axum [`Router`]. This is where the whole HTTP
//! surface of the application is assembled.
//!
//! ## State flow
//!
//! The `AppState` passed to [`build_router`] is used twice:
//!
//! 1. **By sub-routers that need stateful middleware** — `/api/v1` clones
//!    it to attach `from_fn_with_state` layers. See the docs in
//!    [`crate::api::routes::v1`] for why.
//! 2. **By every handler** — the final `.with_state(state)` attaches the
//!    state to the router, where it becomes available via the
//!    [`axum::extract::State`] extractor in any handler.
//!
//! Both uses read from the same value. Handlers and middleware therefore
//! see identical pools, settings, and services.
//!
//! ## Layer ordering
//!
//! `TraceLayer` is applied **last**, making it the outermost layer. That
//! is deliberate: it means even a request that is rejected by the rate
//! limiter or by auth still produces a trace span. Without this, the
//! requests we most want to debug (rejections) would be the ones we see
//! least.

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::api::handlers::health_handler::health_check;
use crate::api::routes::v1;
use crate::app::state::AppState;

/// Builds the application router.
///
/// # Routes
///
/// - `GET /health` — liveness check (unversioned, unauthenticated, not
///   rate-limited).
/// - `/api/v1/*` — the versioned business API. Rate limiting and auth are
///   applied inside the v1 subtree.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Operational endpoint: not part of the versioned API.
        // Load balancers and orchestrators hit this directly, so we do
        // not gate it behind auth or rate limiting — the probe must be
        // as cheap and reliable as possible.
        .route("/health", get(health_check))
        // Versioned business API. `state.clone()` here is a cheap Arc
        // copy; the original `state` is still owned by this function and
        // is used again at `.with_state` below.
        .nest("/api/v1", v1::routes(state.clone()))
        // Tracing wraps everything. See the module docs for why this is
        // applied last.
        .layer(TraceLayer::new_for_http())
        // Make `AppState` available to every handler via the `State`
        // extractor. Must come after all routes are registered.
        .with_state(state)
}