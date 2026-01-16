use anyhow::Result;
use tracing::{error, info};

mod behavior_analyzer;
mod config;
mod daemon;
mod dbus_impl;
mod edge_case_handler;
mod notification_manager;
mod profile_manager;
mod session_manager;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("Starting DOTS Family Daemon");

    if let Err(e) = daemon::run().await {
        error!("Daemon error: {}", e);
        return Err(e);
    }

    info!("DOTS Family Daemon stopped");
    Ok(())
}
