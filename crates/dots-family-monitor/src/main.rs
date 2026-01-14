use anyhow::Result;
use dots_family_monitor::monitor;
use tracing::{error, info};

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

    info!("Starting DOTS Family Monitor");

    if let Err(e) = monitor::run().await {
        error!("Monitor error: {}", e);
        return Err(e);
    }

    info!("DOTS Family Monitor stopped");
    Ok(())
}
