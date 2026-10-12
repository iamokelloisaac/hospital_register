//! # Settings
//!
//! Layered configuration loading.
//!
//! ## Precedence (highest wins)
//!
//! 1. **Environment variables** (from `.env` or the real environment).
//!    These override everything. Used for secrets and per-deployment tunables.
//! 2. **`config/{APP_ENV}.toml`** — environment-specific overrides
//!    (e.g., `config/development.toml`).
//! 3. **`config/default.toml`** — baseline values shared across environments.
//!
//! ## Why split config and secrets?
//!
//! Non-secret configuration (pool sizes, timeouts, feature flags) is
//! committed to the repository in TOML files, where it can be reviewed and
//! versioned. Secrets (`DATABASE_URL`, `REDIS_URL`, `JWT_SECRET`) are read
//! **only** from the environment, never from files that could be accidentally
//! committed.
//!
//! ## References
//!
//! - [`config` crate](https://docs.rs/config/latest/config/) — layered
//!   configuration builder.
//! - [`secrecy` crate](https://docs.rs/secrecy/latest/secrecy/) — wraps
//!   sensitive values to prevent accidental logging.

use serde::Deserialize;
use std::env;

/// Top-level application settings.
///
/// Constructed once at startup via [`Settings::load`] and stored in
/// [`AppState`](crate::app::state::AppState). It is never mutated.
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub app: AppConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub rate_limit: RateLimitConfig,
    pub telemetry: TelemetryConfig,

    /// Secrets are populated separately and are not deserialized from TOML.
    #[serde(skip)]
    pub secrets: Secrets,
}

/// Application-level configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// Human-readable application name.
    pub name: String,
    /// TCP port the HTTP server binds to.
    pub port: u16,
    /// Environment name: `"development"`, `"staging"`, or `"production"`.
    pub env: String,
}

/// Postgres connection pool configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Maximum number of connections the pool will ever open.
    ///
    /// This must be less than Postgres's own `max_connections` setting,
    /// minus headroom for administrative connections. A common rule of
    /// thumb is `max_connections = (2 * cpu_cores) + effective_spindle_count`.
    pub max_connections: u32,

    /// Minimum number of connections to keep warm.
    ///
    /// A background task maintains at least this many open connections, so
    /// the first request after an idle period does not pay connection cost.
    pub min_connections: u32,

    /// How long to wait for a connection from the pool before giving up.
    pub acquire_timeout_secs: u64,

    /// How long a connection may sit idle before being closed.
    pub idle_timeout_secs: u64,

    /// Maximum total lifetime of a connection, regardless of activity.
    pub max_lifetime_secs: u64,
}

/// Redis connection configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    /// Maximum number of connections in the Redis pool.
    pub pool_size: u32,

    /// How long to wait for a Redis connection before giving up.
    pub connection_timeout_secs: u64,
}

/// JWT issuance and validation parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    /// Lifetime of an access token, in minutes.
    pub access_ttl_minutes: i64,

    /// Lifetime of a refresh token, in days.
    pub refresh_ttl_days: i64,
}

/// Rate limiting thresholds.
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Global cap: requests per second per IP.
    pub global_requests_per_second: u32,

    /// Login attempts per minute per IP.
    pub login_requests_per_minute: u32,

    /// Registration attempts per minute per IP.
    pub register_requests_per_minute: u32,
}

/// Telemetry configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryConfig {
    /// Service name reported to tracing backends.
    pub service_name: String,

    /// Log format: `"pretty"` or `"json"`.
    pub log_format: String,
}

/// Secrets that must come from the environment.
#[derive(Debug, Clone, Default)]
pub struct Secrets {
    /// Full Postgres connection string.
    pub database_url: String,

    /// Full Redis connection string.
    pub redis_url: String,

    /// Secret used to sign JWTs. Must be at least 32 bytes of entropy.
    pub jwt_secret: String,
}

impl Settings {
    /// Loads settings from disk and the environment.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A required config file is missing or malformed.
    /// - A required environment variable is not set.
    /// - The merged configuration fails to deserialize.
    pub fn load() -> anyhow::Result<Self> {
        // Determine the environment early: it decides which override file
        // to load. Default to "development" if unset.
        let app_env = env::var("APP_ENV")
            .unwrap_or_else(|_| "development".to_string());

        // Build the layered configuration. `config::File::with_name`
        // automatically appends `.toml` and resolves relative to the
        // current working directory.
        let builder = config::Config::builder()
            .add_source(
                config::File::with_name("config/default"),
            )
            .add_source(
                config::File::with_name(&format!("config/{app_env}"))
                    .required(false),
            );

        let mut settings: Settings = builder.build()?.try_deserialize()?;

        // Secrets come from the environment only. We read them explicitly
        // rather than letting the `config` crate merge them, so that a
        // missing secret is a hard error with a clear message.
        settings.secrets = Secrets {
            database_url: env::var("DATABASE_URL").map_err(|_| {
                anyhow::anyhow!("DATABASE_URL is not set")
            })?,
            redis_url: env::var("REDIS_URL").map_err(|_| {
                anyhow::anyhow!("REDIS_URL is not set")
            })?,
            jwt_secret: env::var("JWT_SECRET").map_err(|_| {
                anyhow::anyhow!("JWT_SECRET is not set")
            })?,
        };

        // Allow `APP_PORT` to override the port from TOML. This is useful
        // for running multiple instances during local development.
        if let Ok(port) = env::var("APP_PORT") {
            if let Ok(parsed) = port.parse::<u16>() {
                settings.app.port = parsed;
            }
        }

        Ok(settings)
    }
}