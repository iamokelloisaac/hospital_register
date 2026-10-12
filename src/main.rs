//! # hospital_register — entry point
//!
//! This is the **composition root**: the one place in the application that
//! knows about every layer and wires them together.
//!
//! ## Startup sequence
//!
//! 1. Load `.env` (if present).
//! 2. Initialize telemetry — **must be first**, or early log lines are lost.
//! 3. Load settings from TOML + environment.
//! 4. Connect to Postgres and Redis.
//! 5. Build `AppState` and the router.
//! 6. Bind a TCP listener and serve.
//!
//! If any step fails, the process exits with a non-zero status and an error
//! message. A server that cannot reach its database should not start.

use std::net::SocketAddr;

use hospital_register::app::router::build_router;
use hospital_register::app::state::AppState;
use hospital_register::config::settings::Settings;
use hospital_register::db::pool;
use hospital_register::shared::telemetry;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Load environment variables from `.env`. `dotenv()` returns `Err`
    //    if the file is missing, but that is fine in production where
    //    variables come from the orchestrator. `.ok()` discards the error.
    dotenvy::dotenv().ok();

    // 2. Initialize telemetry before anything else logs.
    telemetry::init_telemetry();

    // 3. Load layered settings.
    let settings = Settings::load()?;
    tracing::info!(
        env = %settings.app.env,
        "settings loaded"
    );

    // 4. Connect to Postgres and Redis. This also verifies connectivity.
    let pools = pool::build(&settings).await?;

    // 5. Build the application state and router.
    let state = AppState::new(pools, settings.clone());
    let app = build_router(state);

    // 6. Bind and serve.
    let address = SocketAddr::from(([0, 0, 0, 0], settings.app.port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!("🩺 hospital_register listening on http://{}", address);

    // `into_make_service_with_connect_info` makes the peer's socket address
    // available to extractors like `ConnectInfo<SocketAddr>`. Without this,
    // the rate-limit middleware cannot read the client IP from the TCP
    // layer and fails to compile.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
        .await?;

    Ok(())
}