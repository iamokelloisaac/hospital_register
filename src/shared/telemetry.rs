//! # Telemetry
//!
//! Bootstraps [`tracing`], the structured logging and diagnostics framework
//! that the entire application uses.
//!
//! ## Why initialize tracing first?
//!
//! `tracing` uses a **global subscriber**. Whichever subscriber is installed
//! first wins; subsequent installations are ignored (and log a warning).
//! Therefore, [`init_telemetry`] must be the first thing called in `main`,
//! before any other code can emit a log line.
//!
//! ## Format selection
//!
//! - **Development:** a human-readable "pretty" format with colors, timestamps,
//!   and source locations. Ideal for reading in a terminal.
//! - **Production:** structured JSON, one object per line. Ideal for ingestion
//!   by log aggregators (Loki, Elasticsearch, Datadog, etc.).
//!
//! The format is chosen based on the `APP_ENV` environment variable. If it
//! equals `"production"` (case-insensitive), JSON is used; otherwise, pretty.
//!
//! ## References
//!
//! - [`tracing_subscriber::EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)
//!   — parses `RUST_LOG` directives like `my_crate=debug,info`.
//! - [`tracing_subscriber::fmt::layer`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html)
//!   — provides the `pretty()` and `json()` format layers.

use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initializes the global tracing subscriber.
///
/// # Panics
///
/// Panics if called more than once, or if another global subscriber has
/// already been installed. This is intentional: telemetry setup happens once,
/// at startup, and a second call indicates a programming error.
///
/// # Example
///
/// ```no_run
/// use hospital_register::shared::telemetry;
///
/// fn main() {
///     telemetry::init_telemetry();
///     tracing::info!("this line will be captured");
/// }
/// ```
pub fn init_telemetry() {
    // `EnvFilter` reads the `RUST_LOG` environment variable and parses
    // directives like `my_crate=debug,tower_http=trace,info`. If `RUST_LOG`
    // is unset or malformed, we fall back to a sensible default that shows
    // our own crate's debug logs, tower-http's debug logs, and everything
    // else at `info`.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(
                "hospital_register=debug,tower_http=debug,info",
            )
        });

    // Determine whether we are in production. We read `APP_ENV` directly
    // from the environment here (rather than from `Settings`) because
    // telemetry must be initialized *before* settings are loaded — otherwise
    // we would miss any log lines emitted during settings loading.
    let is_production = std::env::var("APP_ENV")
        .map(|value| value.eq_ignore_ascii_case("production"))
        .unwrap_or(false);

    // `registry()` creates a `Registry` subscriber, which is the base onto
    // which we stack layers. This is the recommended pattern for composing
    // multiple layers (filter + format).
    let registry = tracing_subscriber::registry().with(env_filter);

    if is_production {
        // JSON output: one object per line, machine-parseable.
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        // Pretty output: human-readable, colored, with source locations.
        registry
            .with(tracing_subscriber::fmt::layer().pretty())
            .init();
    }
}