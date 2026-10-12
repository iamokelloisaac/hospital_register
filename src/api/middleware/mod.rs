//! # Tower middleware
//!
//! Reusable request-processing layers applied to the router.
//!
//! ## What lives here
//!
//! - [`auth`] — verifies JWTs from the `Authorization` header and injects
//!   the decoded [`Claims`](crate::services::auth_service::Claims) into
//!   request extensions.
//! - [`rate_limit`] — applies a Redis-backed token-bucket limit per client
//!   IP. Different buckets are used for global traffic, login, and
//!   registration.
//!
//! ## Why middleware and not handler code?
//!
//! Cross-cutting concerns like authentication and rate limiting should be
//! applied uniformly, not repeated in every handler. If a new protected
//! route is added and the developer forgets to check the token, that is a
//! security bug. Attaching the check as middleware to a subtree makes the
//! safe path the default path: new routes added under the protected
//! subtree are protected automatically.
//!
//! ## Axum middleware signatures
//!
//! Middleware written with [`axum::middleware::from_fn`] takes the form:
//!
//! ```ignore
//! async fn my_middleware(
//!     State(state): State<AppState>,   // optional; only if state is needed
//!     request: Request,                // the incoming request
//!     next: Next,                      // the rest of the chain
//! ) -> Response { /* ... */ }
//! ```
//!
//! Extractor arguments before `Request` and `Next` are resolved against
//! the router's state, so state access is available exactly as in a
//! handler.

pub mod auth;
pub mod rate_limit;