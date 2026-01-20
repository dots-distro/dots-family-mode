use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use zbus::ConnectionBuilder;

use crate::config::DaemonConfig;
use crate::dbus_impl::FamilyDaemonService;
use crate::ebpf::{EbpfHealth, EbpfManager};
use crate::edge_case_handler::EdgeCaseHandler;
use crate::enforcement::EnforcementEngine;
use crate::monitoring_service::MonitoringService;
use crate::policy_engine::PolicyEngine;
use crate::profile_manager::ProfileManager;
use dots_family_db::{migrations, Database, DatabaseConfig};

pub struct Daemon {
    ebpf_manager: RwLock<Option<EbpfManager>>,
    policy_engine: RwLock<PolicyEngine>,
    enforcement_engine: RwLock<EnforcementEngine>,
    config: DaemonConfig,
}

impl Daemon {
    pub async fn new() -> Result<Self> {
        info!("Initializing daemon");

        let config = DaemonConfig::load()?;
        let policy_engine =
            PolicyEngine::new().await.context("Failed to initialize policy engine")?;
        let enforcement_engine = EnforcementEngine::new(config.dry_run.unwrap_or(false));

        Ok(Self {
            ebpf_manager: RwLock::new(None),
            policy_engine: RwLock::new(policy_engine),
            enforcement_engine: RwLock::new(enforcement_engine),
            config,
        })
    }

    pub async fn set_ebpf_manager(&self, manager: EbpfManager) {
        let mut ebpf_manager = self.ebpf_manager.write().await;
        *ebpf_manager = Some(manager);
    }

    pub async fn get_policy_engine(&self) -> tokio::sync::RwLockReadGuard<'_, PolicyEngine> {
        self.policy_engine.read().await
    }

    pub async fn get_policy_engine_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, PolicyEngine> {
        self.policy_engine.write().await
    }

    pub async fn get_enforcement_engine(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, EnforcementEngine> {
        self.enforcement_engine.read().await
    }

    #[allow(dead_code)]
    pub async fn get_enforcement_engine_mut(
        &self,
    ) -> tokio::sync::RwLockWriteGuard<'_, EnforcementEngine> {
        self.enforcement_engine.write().await
    }

    pub async fn get_ebpf_health(&self) -> Option<EbpfHealth> {
        let ebpf_manager = self.ebpf_manager.read().await;
        if let Some(ref manager) = *ebpf_manager {
            Some(manager.get_health_status().await)
        } else {
            None
        }
    }
}

pub async fn initialize_database() -> Result<Database> {
    info!("Initializing database");

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "/tmp/dots-family.db".to_string());

    migrations::create_database_if_not_exists(&database_url)
        .await
        .context("Failed to create database")?;

    let database_config = DatabaseConfig { path: database_url, encryption_key: None };
    let database = Database::new(database_config).await.context("Failed to connect to database")?;

    migrations::run_migrations(database.pool()?).await.context("Failed to run migrations")?;

    info!("Database initialized successfully");
    Ok(database)
}

pub async fn run() -> Result<()> {
    info!("Initializing daemon");

    // Initialize database first
    let database = initialize_database().await?;

    let daemon = Arc::new(Daemon::new().await?);
    info!("Daemon with policy engine initialized successfully");

    let ebpf_manager = match EbpfManager::new().await {
        Ok(mut manager) => {
            info!("eBPF manager initialized successfully");

            // Load eBPF programs if manager is available
            if let Err(e) = manager.load_all_programs().await {
                error!("Failed to load eBPF programs: {}", e);
            } else {
                let status = manager.get_health_status().await;
                info!(
                    "eBPF programs loaded: {}/{} healthy",
                    status.programs_loaded,
                    if status.all_healthy { "all" } else { "some" }
                );
            }

            // Set eBPF manager in daemon
            daemon.set_ebpf_manager(manager).await;
            true
        }
        Err(e) => {
            error!("Failed to initialize eBPF manager: {}", e);
            false
        }
    };

    let monitoring_service = MonitoringService::new().await?;

    // Create ProfileManager with shared database instance
    let profile_manager = ProfileManager::new(&daemon.config, database).await?;

    let service = FamilyDaemonService::new_with_daemon(
        &daemon.config,
        monitoring_service.clone(),
        daemon.clone(),
        profile_manager.clone(),
    )
    .await?;

    let mut edge_case_handler = EdgeCaseHandler::new();
    edge_case_handler.start_monitoring().await?;

    monitoring_service.start().await?;

    let conn_builder = if daemon.config.dbus.use_session_bus {
        info!("Using session bus for development mode");
        ConnectionBuilder::session()?
    } else {
        info!("Using system bus for production mode");
        ConnectionBuilder::system()?
    };

    let conn = conn_builder
        .name(daemon.config.dbus.service_name.as_str())?
        .serve_at("/org/dots/FamilyDaemon", service)?
        .build()
        .await?;

    info!("DBus service registered at {}", daemon.config.dbus.service_name);
    if ebpf_manager {
        info!("eBPF monitoring service started");
    } else {
        warn!("eBPF monitoring service not available - running in degraded mode");
    }

    let conn_clone = conn.clone();
    let daemon_clone_enforcement = daemon.clone();
    tokio::spawn(async move {
        let mut interval_timer = interval(Duration::from_secs(30));
        let mut last_warning_time: Option<u32> = None;

        loop {
            interval_timer.tick().await;

            if let Err(e) = enforce_time_limits(
                &profile_manager,
                &conn_clone,
                &daemon_clone_enforcement.config.dbus.service_name,
                &mut last_warning_time,
            )
            .await
            {
                warn!("Policy enforcement error: {}", e);
            }
        }
    });

    let daemon_clone_policy = daemon.clone();
    let monitoring_service_clone = monitoring_service.clone();
    tokio::spawn(async move {
        let mut interval_timer = interval(Duration::from_secs(5));

        loop {
            interval_timer.tick().await;

            if let Err(e) =
                process_activity_enforcement(&daemon_clone_policy, &monitoring_service_clone).await
            {
                warn!("Activity processing error: {}", e);
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
    service_name: &str,
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

                    if let Err(e) = emit_time_warning(conn, service_name, remaining).await {
                        warn!("Failed to emit time warning signal: {}", e);
                    } else {
                        *last_warning_time = Some(remaining);
                    }
                } else if remaining == 0 && *last_warning_time != Some(0) {
                    warn!("Time limit exceeded for profile: {}", profile.name);

                    if let Err(e) = emit_time_warning(conn, service_name, 0).await {
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

async fn emit_time_warning(
    conn: &zbus::Connection,
    service_name: &str,
    minutes_remaining: u32,
) -> Result<()> {
    conn.emit_signal(
        None::<()>,
        "/org/dots/FamilyDaemon",
        service_name,
        "TimeLimitWarning",
        &minutes_remaining,
    )
    .await?;

    info!("Emitted TimeLimitWarning signal: {} minutes", minutes_remaining);
    Ok(())
}

async fn process_activity_enforcement(
    daemon: &Arc<Daemon>,
    monitoring_service: &MonitoringService,
) -> Result<()> {
    let activities = monitoring_service.get_recent_activities().await?;

    if activities.is_empty() {
        return Ok(());
    }

    let mut policy_engine = daemon.get_policy_engine_mut().await;
    let enforcement_engine = daemon.get_enforcement_engine().await;

    for activity in activities {
        policy_engine.update_activity();

        let activity_clone = activity.clone();
        let (app_id, pid) = match &activity_clone {
            dots_family_proto::events::ActivityEvent::WindowFocused { app_id, pid, .. } => {
                (Some(app_id.as_str()), Some(*pid))
            }
            dots_family_proto::events::ActivityEvent::ProcessStarted {
                executable, pid, ..
            } => (Some(executable.split('/').next_back().unwrap_or(executable)), Some(*pid)),
            _ => (None, None),
        };

        match policy_engine.process_activity(activity).await {
            Ok(decision) => {
                if decision.blocked {
                    warn!("Blocking activity: {} - {}", decision.action, decision.reason);

                    if let Err(e) =
                        enforcement_engine.enforce_policy_decision(&decision, app_id, pid).await
                    {
                        error!("Failed to enforce policy decision: {}", e);
                    }
                } else {
                    debug!("Allowing activity: {} - {}", decision.action, decision.reason);
                }
            }
            Err(e) => {
                error!("Policy processing error: {}", e);
            }
        }
    }

    Ok(())
}
