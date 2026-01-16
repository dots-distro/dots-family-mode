use anyhow::Result;
use tokio::signal;
use tokio::time::{interval, Duration};
use tracing::{info, warn};
use zbus::ConnectionBuilder;

use crate::config::DaemonConfig;
use crate::dbus_impl::FamilyDaemonService;
use crate::edge_case_handler::EdgeCaseHandler;
use crate::monitoring_service::MonitoringService;
use crate::profile_manager::ProfileManager;

pub async fn run() -> Result<()> {
    info!("Initializing daemon");

    let config = DaemonConfig::load()?;
    let monitoring_service = MonitoringService::new();

    let service = FamilyDaemonService::new(&config, monitoring_service.clone()).await?;
    let profile_manager = ProfileManager::new(&config).await?;

    let mut edge_case_handler = EdgeCaseHandler::new();
    edge_case_handler.start_monitoring().await?;

    monitoring_service.start().await?;

    let conn = ConnectionBuilder::system()?
        .name("org.dots.FamilyDaemon")?
        .serve_at("/org/dots/FamilyDaemon", service)?
        .build()
        .await?;

    info!("DBus service registered at org.dots.FamilyDaemon");
    info!("eBPF monitoring service started");

    let conn_clone = conn.clone();
    tokio::spawn(async move {
        let mut interval_timer = interval(Duration::from_secs(30));
        let mut last_warning_time: Option<u32> = None;

        loop {
            interval_timer.tick().await;

            if let Err(e) =
                enforce_time_limits(&profile_manager, &conn_clone, &mut last_warning_time).await
            {
                warn!("Policy enforcement error: {}", e);
            }
        }
    });

    info!("Daemon running with policy enforcement, waiting for shutdown signal...");

    #[cfg(unix)]
    {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down gracefully...");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down gracefully...");
            }
            _ = signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down gracefully...");
            }
        }
    }

    #[cfg(not(unix))]
    {
        signal::ctrl_c().await?;
        info!("Received Ctrl+C, shutting down gracefully...");
    }

    monitoring_service.stop().await?;
    info!("Monitoring service stopped");

    info!("Daemon shutdown complete");

    Ok(())
}

async fn enforce_time_limits(
    profile_manager: &ProfileManager,
    conn: &zbus::Connection,
    last_warning_time: &mut Option<u32>,
) -> Result<()> {
    if let Ok(Some(profile)) = profile_manager.get_active_profile().await {
        match profile_manager.get_remaining_time().await {
            Ok(remaining) => {
                if remaining <= 5 && remaining > 0 && *last_warning_time != Some(remaining) {
                    info!(
                        "Time limit warning: {} minutes remaining for profile: {}",
                        remaining, profile.name
                    );

                    if let Err(e) = emit_time_warning(conn, remaining).await {
                        warn!("Failed to emit time warning signal: {}", e);
                    } else {
                        *last_warning_time = Some(remaining);
                    }
                } else if remaining == 0 && *last_warning_time != Some(0) {
                    warn!("Time limit exceeded for profile: {}", profile.name);

                    if let Err(e) = emit_time_warning(conn, 0).await {
                        warn!("Failed to emit time exceeded signal: {}", e);
                    } else {
                        *last_warning_time = Some(0);
                    }
                }
            }
            Err(e) => warn!("Failed to check remaining time: {}", e),
        }
    }
    Ok(())
}

async fn emit_time_warning(conn: &zbus::Connection, minutes_remaining: u32) -> Result<()> {
    conn.emit_signal(
        None::<()>,
        "/org/dots/FamilyDaemon",
        "org.dots.FamilyDaemon",
        "TimeLimitWarning",
        &minutes_remaining,
    )
    .await?;

    info!("Emitted TimeLimitWarning signal: {} minutes", minutes_remaining);
    Ok(())
}
