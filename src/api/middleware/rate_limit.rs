//! # Rate limiting middleware
//!
//! Applies a Redis-backed fixed-window limiter per client.
//!
//! ## Failure mode
//!
//! If Redis is unreachable, this middleware **fails open**: the request is
//! allowed through and a warning is logged. The alternative (fail closed)
//! would turn a Redis outage into a total API outage, which is worse than
//! briefly losing rate limiting. In a system where rate limiting protects
//! against abuse and nothing else, fail-open is the right default. If we
//! ever add billing-grade limits, this decision should be revisited.
//!
//! ## Response on rejection
//!
//! A rejected request returns `429 Too Many Requests` with:
//!
//! - a JSON body: `{ "error": "rate limit exceeded" }`
//! - a `Retry-After` header set to the number of seconds until the window
//!   resets. This is the standard signal that polite clients (and HTTP
//!   libraries like `reqwest`) know to honour.

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::net::SocketAddr;

use crate::app::state::AppState;
use crate::services::rate_limit_service::{
    client_identifier, RateLimitBucket,
};

/// Middleware that applies the global per-IP rate limit.
///
/// The global bucket is deliberately loose (100 requests per second). It
/// exists to stop floods and scraping, not to enforce business policy.
/// Business-sensitive routes (login, registration) get their own tighter
/// buckets, applied by their own middleware instances.
pub async fn global_rate_limit(
    State(state): State<AppState>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    apply_limit(
        &state,
        RateLimitBucket::Global,
        connection_info,
        request,
        next,
    )
        .await
}

/// Middleware that applies the login-specific rate limit.
///
/// Attach this only to the `/auth/login` route. It uses a tighter bucket
/// (5 requests per minute per IP) to slow down credential-stuffing.
pub async fn login_rate_limit(
    State(state): State<AppState>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    apply_limit(
        &state,
        RateLimitBucket::Login,
        connection_info,
        request,
        next,
    )
        .await
}

/// Shared implementation for both buckets. Kept private so that adding a
/// third bucket is a one-line change: define the public `fn` above and
/// reuse this body.
async fn apply_limit(
    state: &AppState,
    bucket: RateLimitBucket,
    connection_info: SocketAddr,
    request: Request,
    next: Next,
) -> Response {
    let identifier = client_identifier(
        request.headers(),
        Some(connection_info),
    );

    match state
        .rate_limit_service
        .check_for(bucket, &identifier)
        .await
    {
        Ok(decision) if decision.allowed => {
            // Within budget. Continue down the chain. We could also add
            // `X-RateLimit-Remaining` response headers here for visibility
            // — a nice-to-have, not required by the spec.
            next.run(request).await
        }
        Ok(decision) => {
            // Over budget. Build the rejection.
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "rate limit exceeded",
                    "retry_after_seconds": decision.retry_after_seconds,
                })),
            )
                .into_response();

            // `Retry-After` is a standard header. Clients and proxies
            // understand it without any further convention on our side.
            if let Ok(value) =
                HeaderValue::from_str(&decision.retry_after_seconds.to_string())
            {
                response.headers_mut().insert("retry-after", value);
            }

            response
        }
        Err(error) => {
            // Fail open. Log loudly; allow the request. A rate limiter
            // outage should not take the API down with it.
            tracing::warn!(
                error = %error,
                "rate limiter unavailable; allowing request through"
            );
            next.run(request).await
        }
    }
}