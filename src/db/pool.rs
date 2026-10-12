//! # Connection pools
//!
//! Constructs and validates the two external connections the application
//! depends on: a Postgres connection pool and a Redis connection manager.
//!
//! ## Why pool Postgres?
//!
//! Opening a Postgres connection involves a TCP handshake, TLS negotiation,
//! and authentication — roughly 50–100 ms. A pool opens connections once and
//! reuses them, reducing per-request latency to sub-millisecond levels. It
//! also caps the total number of connections, protecting the database from
//! being overwhelmed.
//!
//! ## Why `ConnectionManager` for Redis?
//!
//! Redis's `ConnectionManager` is a proxy around a *multiplexed* connection.
//! It is cheaply cloneable, so every task gets its own handle, and it
//! automatically reconnects in the background if the connection drops. This
//! is the recommended way to use Redis from async Rust.
//!
//! ## References
//!
//! - [`sqlx::postgres::PgPoolOptions`](https://docs.rs/sqlx/latest/sqlx/postgres/struct.PgPoolOptions.html)
//! - [`redis::aio::ConnectionManager`](https://docs.rs/redis/latest/redis/aio/struct.ConnectionManager.html)

use std::time::Duration;

use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::settings::Settings;

/// Holds both connection handles.
///
/// Both `PgPool` and `ConnectionManager` are internally reference-counted,
/// so cloning `DbPools` is cheap and gives each holder an independent handle
/// to the same underlying connections.
#[derive(Clone)]
pub struct DbPools {
    /// Postgres connection pool.
    pub pg: PgPool,

    /// Redis connection manager. Clone it per task.
    pub redis: ConnectionManager,
}

/// Builds both pools and verifies connectivity.
///
/// # Errors
///
/// Returns an error if either Postgres or Redis cannot be reached. The
/// application should refuse to start in that case: a hospital register that
/// cannot talk to its database is worse than one that is down, because it
/// would accept requests it cannot fulfil.
pub async fn build(settings: &Settings) -> anyhow::Result<DbPools> {
    // --- Postgres ---
    //
    // `PgPoolOptions` configures the pool's behavior. Every option here is
    // sourced from `Settings`, which in turn comes from TOML + env vars.
    let pg = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .acquire_timeout(Duration::from_secs(
            settings.database.acquire_timeout_secs,
        ))
        .idle_timeout(Duration::from_secs(
            settings.database.idle_timeout_secs,
        ))
        .max_lifetime(Duration::from_secs(
            settings.database.max_lifetime_secs,
        ))
        .connect(&settings.secrets.database_url)
        .await?;

    // Verify the connection with a trivial query. If this fails, we want to
    // know now, not on the first user request.
    sqlx::query("SELECT 1").execute(&pg).await?;
    tracing::info!("connected to Postgres");

    // --- Redis ---
    //
    // `Client::open` parses the connection URL. It does not connect yet.
    let redis_client = redis::Client::open(settings.secrets.redis_url.clone())?;

    // `ConnectionManager::new` establishes the initial connection and
    // returns a handle that reconnects automatically on failure.
    let redis = ConnectionManager::new(redis_client).await?;

    // Verify with PING.
    let mut conn = redis.clone();
    let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
    if pong != "PONG" {
        anyhow::bail!("unexpected Redis PING response: {pong}");
    }
    tracing::info!("connected to Redis");

    Ok(DbPools { pg, redis })
}