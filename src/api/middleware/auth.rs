//! # Auth middleware
//!
//! Verifies the JWT from the `Authorization` header, injects the decoded
//! claims into request extensions, and lets the request continue.
//!
//! ## How handlers access claims
//!
//! After this middleware runs, any handler downstream can declare
//! `Extension<Claims>` as an argument and receive the verified claims:
//!
//! ```ignore
//! async fn my_handler(
//!     Extension(claims): Extension<Claims>,
//! ) -> impl IntoResponse { /* ... */ }
//! ```
//!
//! The `Extension` extractor reads from `request.extensions()`, which is
//! the same map we insert into below. The type is the key: `Extension<T>`
//! looks up the extension by the concrete Rust type `T`.
//!
//! ## Why not check auth inside handlers?
//!
//! Because forgetting the check on a new route is a security bug. By
//! attaching auth as a middleware layer on the whole protected subtree,
//! every new route is protected the moment it exists. Making a route
//! public requires deliberately moving it out of the protected subtree —
//! a visible code change, reviewable at a glance.
//!
//! ## Signature and the state generic
//!
//! This middleware takes `State<AppState>` because it needs the
//! `AuthService` to verify tokens. That makes it *stateful*, and stateful
//! middleware in Axum 0.8 must be attached with
//! [`axum::middleware::from_fn_with_state`], not `from_fn`. See
//! [`crate::api::routes::v1`] for the wiring.

use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::app::state::AppState;

/// Middleware that requires a valid Bearer token.
///
/// Rejects with `401 Unauthorized` if the header is missing, malformed,
/// or the token fails verification.
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract the `Authorization: Bearer <token>` header.
    //
    // `strip_prefix` returns `Option<&str>`, which is exactly the shape we
    // want: everything after `"Bearer "`, or `None` if the header has a
    // different scheme. We do not accept a bare token without the scheme.
    let token = match request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    {
        Some(token) => token.to_string(),
        None => {
            return unauthorized("missing or malformed Authorization header")
        }
    };

    // Verify the token. `verify_access_token` returns `CoreError` on
    // failure; we do not care which variant, because every failure maps
    // to the same HTTP response.
    match state.auth_service.verify_access_token(&token) {
        Ok(claims) => {
            // Attach the verified claims to request extensions. Downstream
            // handlers can pull them out with `Extension<Claims>`. The
            // type is the key, so no explicit registration is needed.
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(_) => unauthorized("invalid or expired token"),
    }
}

/// Builds a standard `401` response. Kept private to this module: no
/// other code should be producing this particular shape.
fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": message })),
    )
        .into_response()
}