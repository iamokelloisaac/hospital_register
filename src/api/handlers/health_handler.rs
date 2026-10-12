//! # Health check handler
//!
//! `GET /health` — reports whether the service and its dependencies are
//! operational.
//!
//! ## Why a real health check?
//!
//! A load balancer or orchestrator (Kubernetes, Nomad) uses this endpoint
//! to decide whether to route traffic to an instance. A handler that
//! returns a hardcoded `"OK"` would let an instance that cannot reach its
//! database stay in rotation — silently failing every request. This
//! handler actually pings both dependencies.
//!
//! ## Response shape
//!
//! - `200 OK` and `{"status":"ok",...}` when Postgres and Redis are both
//!   reachable.
//! - `503 Service Unavailable` and `{"status":"degraded",...}` when either
//!   is down. The offending dependency is marked `"down"` so dashboards
//!   and alerts can distinguish "database down" from "cache down".

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::app::state::AppState;

/// `GET /health`
pub async fn health_check(
    State(state): State<AppState>,
) -> impl IntoResponse {
    // --- Postgres ---
    //
    // `SELECT 1` is the canonical liveness probe: it exercises the
    // connection without touching any user data. If the pool is exhausted
    // or Postgres is unreachable, this returns `Err` and we mark the
    // dependency as down.
    let database_ok = sqlx::query("SELECT 1")
        .execute(&state.pools.pg)
        .await
        .is_ok();

    // --- Redis ---
    //
    // `ConnectionManager` is cloneable; clone to get a mutable handle for
    // this call, since `query_async` takes `&mut C`.
    //
    // On the generic arguments: in `redis` 0.25, `query_async` is generic
    // over both the connection type (`C`) and the reply type (`T`). The
    // turbofish `::<_, String>` lets the compiler infer `C` from
    // `&mut redis_connection` while fixing `T` to `String`. Supplying
    // only one generic would be ambiguous — the compiler would treat it
    // as `C`, not `T`, and the request would not type-check.
    let mut redis_connection = state.pools.redis.clone();
    let redis_ok = match redis::cmd("PING")
        .query_async::<_, String>(&mut redis_connection)
        .await
    {
        Ok(response) => response == "PONG",
        Err(error) => {
            // Log the error so operators can see *why* the check failed.
            // We do not surface it in the response: the client only needs
            // to know "up" or "down", not the internal error message.
            tracing::warn!(error = %error, "redis ping failed");
            false
        }
    };

    let all_ok = database_ok && redis_ok;

    let status = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = Json(json!({
        "status": if all_ok { "ok" } else { "degraded" },
        "database": if database_ok { "ok" } else { "down" },
        "redis": if redis_ok { "ok" } else { "down" },
        "version": env!("CARGO_PKG_VERSION"),
    }));

    (status, body)
}