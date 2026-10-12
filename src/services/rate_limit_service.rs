//! # Rate limit service
//!
//! A Redis-backed **fixed-window** rate limiter.
//!
//! ## What "fixed window" means
//!
//! The counter for a given key lives for a fixed duration. Within that
//! duration, every request increments the counter by one. When the counter
//! exceeds the limit, further requests are rejected. When the window
//! expires, the counter resets.
//!
//! ## Trade-offs compared to a sliding window
//!
//! A fixed window can allow up to `2 * limit` requests at a window
//! boundary: e.g., 5 requests in the final millisecond of window N, then
//! 5 more in the first millisecond of window N+1. A sliding window avoids
//! this by storing per-request timestamps, at the cost of more memory and
//! more complex atomic operations. For our purposes (protecting against
//! credential stuffing, not enforcing billing), fixed window is the
//! right trade-off: simpler, cheaper, and good enough.
//!
//! ## Atomicity
//!
//! The `INCR` command in Redis is atomic. Two concurrent requests will
//! each see a distinct incremented value. The `EXPIRE` is only set on the
//! first increment (when the returned value is `1`) so we do not reset the
//! TTL on every request — that would turn the fixed window into a
//! perpetually-extending window and defeat the limiter entirely.
//!
//! ## References
//!
//! - [`redis::AsyncCommands`](https://docs.rs/redis/latest/redis/trait.AsyncCommands.html)
//!   — the trait that provides `incr`, `expire`, and `ttl` as async methods.

use std::sync::Arc;

use redis::AsyncCommands;
use uuid::Uuid;

use crate::core::errors::CoreError;
use crate::db::pool::DbPools;

/// The outcome of a rate limit check.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitDecision {
    /// Whether the request is allowed through.
    pub allowed: bool,

    /// Requests remaining in the current window.
    pub remaining: u32,

    /// Seconds until the current window resets. Used to populate the
    /// `Retry-After` header on a rejection.
    pub retry_after_seconds: u64,
}

pub struct RateLimitService {
    pools: Arc<DbPools>,
}

impl RateLimitService {
    pub fn new(pools: Arc<DbPools>) -> Self {
        Self { pools }
    }

    /// Checks and increments a counter for the given `key` in a window of
    /// `window_seconds`, allowing up to `limit` requests per window.
    ///
    /// The `key` is combined with a bucket name (e.g., `"global"` or
    /// `"login"`) so that different limits do not collide. For example,
    /// an IP that exhausts its login attempts does not affect its global
    /// request budget.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] if Redis is unreachable. The caller
    /// (the middleware) decides whether to fail open or fail closed. We
    /// fail open in production because a rate limiter outage should not
    /// take the API down with it.
    pub async fn check(
        &self,
        bucket: &str,
        identifier: &str,
        limit: u32,
        window_seconds: u64,
    ) -> Result<RateLimitDecision, CoreError> {
        // Redis keys are byte strings; we build one per (bucket, identifier)
        // pair. The `{...}` syntax is a Redis Cluster hash tag: it forces
        // the key onto a specific shard. For our single-node setup it is
        // harmless; for a cluster it avoids cross-slot operations.
        let key = format!("ratelimit:{{{bucket}}}:{identifier}");

        // Clone the connection manager for this call. `ConnectionManager`
        // is cheap to clone and multiplexes onto the same underlying
        // socket.
        let mut connection = self.pools.redis.clone();

        // INCR is atomic. On the first call within a window, the new value
        // is 1; Redis creates the key with no TTL. On subsequent calls, the
        // value increases and the TTL is untouched.
        let count: u32 = connection
            .incr(&key, 1u32)
            .await
            .map_err(|error| {
                CoreError::Internal(format!(
                    "rate limit INCR failed: {error}"
                ))
            })?;

        // First request in this window: attach the TTL. Doing this inside
        // the same method as INCR is not fully atomic (a crash between
        // INCR and EXPIRE would leave a key with no TTL), but the failure
        // mode is benign: worst case, one key sticks around until it is
        // evicted by Redis's own memory policy. A Lua script would make it
        // atomic; that is a worthwhile improvement if this becomes a
        // hot path.
        if count == 1 {
            // `let _: () =` is the idiomatic way to discard a Redis reply
            // whose value we do not care about. The `AsyncCommands::expire`
            // method is generic over the reply type, so the annotation
            // tells the compiler which one to expect.
            let _: () = connection
                .expire(&key, window_seconds as i64)
                .await
                .map_err(|error| {
                    CoreError::Internal(format!(
                        "rate limit EXPIRE failed: {error}"
                    ))
                })?;
        }

        // Compute remaining budget. `saturating_sub` prevents underflow if
        // count briefly exceeds limit due to concurrent increments.
        let remaining = limit.saturating_sub(count);

        // TTL tells the client how long to wait before retrying. If Redis
        // returns -1 (no TTL) or -2 (key expired between INCR and TTL), we
        // fall back to the full window length, which is a safe upper bound.
        let ttl: i64 = connection.ttl(&key).await.unwrap_or(window_seconds as i64);
        let retry_after_seconds = if ttl < 0 {
            window_seconds
        } else {
            ttl as u64
        };

        Ok(RateLimitDecision {
            allowed: count <= limit,
            remaining,
            retry_after_seconds,
        })
    }

    /// Convenience wrapper for a specific bucket. Bucket names are
    /// constants in the middleware; this method exists only to make the
    /// calling code read well.
    pub async fn check_for(
        &self,
        bucket: RateLimitBucket,
        identifier: &str,
    ) -> Result<RateLimitDecision, CoreError> {
        let (name, limit, window) = bucket.config();
        self.check(name, identifier, limit, window).await
    }
}

/// Predefined rate limit buckets. Adding a new bucket here forces every
/// caller to name it, which keeps the policy discoverable.
#[derive(Debug, Clone, Copy)]
pub enum RateLimitBucket {
    /// Broad per-IP cap on all API traffic.
    Global,
    /// Tight cap on login attempts, keyed by client IP.
    Login,
    /// Tight cap on registration attempts, keyed by client IP.
    Register,
}

impl RateLimitBucket {
    /// Returns `(bucket_name, requests_allowed, window_seconds)`.
    fn config(self) -> (&'static str, u32, u64) {
        match self {
            RateLimitBucket::Global => ("global", 100, 1),
            RateLimitBucket::Login => ("login", 5, 60),
            RateLimitBucket::Register => ("register", 5, 60),
        }
    }
}

/// Extracts a stable client identifier from request headers and connection
/// info. Returns the client IP as a string, or a sentinel when none is
/// available.
///
/// ## Why a header first?
///
/// In production, our service almost always sits behind a reverse proxy
/// (nginx, Traefik, a cloud load balancer). The direct TCP peer is the
/// proxy, not the user. The proxy sets `X-Forwarded-For` (or
/// `X-Real-IP`), which we read here.
///
/// ## Trust warning
///
/// `X-Forwarded-For` is client-controllable unless the proxy strips and
/// rewrites it. **Only deploy this behind a proxy that does so.** Directly
/// exposing the service to the internet with this logic in place lets
/// attackers spoof any IP they like and bypass the limiter.
pub fn client_identifier(
    headers: &axum::http::HeaderMap,
    connection_info: Option<std::net::SocketAddr>,
) -> String {
    // `X-Forwarded-For` may contain a comma-separated chain: the first
    // entry is the original client, later entries are proxies. We trust
    // the first non-empty one.
    if let Some(value) = headers.get("x-forwarded-for") {
        if let Ok(text) = value.to_str() {
            if let Some(first) = text.split(',').map(str::trim).find(|s| !s.is_empty()) {
                return first.to_string();
            }
        }
    }

    // Fallback header used by some proxies.
    if let Some(value) = headers.get("x-real-ip") {
        if let Ok(text) = value.to_str() {
            if !text.is_empty() {
                return text.to_string();
            }
        }
    }

    // Fall back to the TCP peer address if we have it.
    if let Some(address) = connection_info {
        return address.ip().to_string();
    }

    // Last resort: a sentinel. All such requests share a bucket, which
    // is stricter than no limit but means we never fail to apply the
    // limiter entirely.
    "unknown".to_string()
}

/// Included for symmetry. Kept as documentation of what a user-scoped
/// limiter would look like: instead of IP, key by authenticated subject.
///
/// We do not use it yet because authentication middleware runs *after*
/// rate limiting (we want to reject floods before doing crypto work).
pub fn authenticated_identifier(subject: Uuid) -> String {
    format!("user:{subject}")
}