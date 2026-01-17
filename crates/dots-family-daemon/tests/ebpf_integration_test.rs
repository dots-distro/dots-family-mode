use anyhow::Result;
use dots_family_daemon::ebpf::EbpfManager;

#[tokio::test]
async fn test_ebpf_manager_initialization() -> Result<()> {
    let manager = EbpfManager::new().await?;

    let health = manager.get_health_status().await;

    assert!(health.program_status.contains_key("process_monitor"));
    assert!(health.program_status.contains_key("network_monitor"));
    assert!(health.program_status.contains_key("filesystem_monitor"));

    Ok(())
}

#[tokio::test]
async fn test_ebpf_programs_loading() -> Result<()> {
    let mut manager = EbpfManager::new().await?;

    manager.load_all_programs().await?;

    let health = manager.get_health_status().await;

    assert_eq!(health.program_status.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_ebpf_health_check() -> Result<()> {
    let mut manager = EbpfManager::new().await?;

    manager.set_health_status_for_test(dots_family_daemon::ebpf::EbpfHealth {
        programs_loaded: 3,
        all_healthy: true,
        program_status: {
            let mut status = std::collections::HashMap::new();
            status.insert("process_monitor".to_string(), true);
            status.insert("network_monitor".to_string(), true);
            status.insert("filesystem_monitor".to_string(), true);
            status
        },
    });

    let status = manager.get_health_status().await;

    assert_eq!(status.programs_loaded, 3);
    assert!(status.all_healthy);

    Ok(())
}

#[tokio::test]
async fn test_health_status_logic_fix() -> Result<()> {
    let manager = EbpfManager::new().await?;
    let status = manager.get_health_status().await;

    let actually_loaded_count = status.program_status.values().filter(|&loaded| *loaded).count();
    let expected_healthy = actually_loaded_count == 3;

    assert_eq!(status.programs_loaded, actually_loaded_count);
    assert_eq!(status.all_healthy, expected_healthy);

    Ok(())
}

#[tokio::test]
async fn test_daemon_ebpf_integration() -> Result<()> {
    // This test verifies daemon initializes with eBPF
    use dots_family_daemon::daemon::Daemon;

    let daemon = Daemon::new().await?;
    assert!(daemon.get_ebpf_health().await.is_some(), "Daemon should initialize with eBPF support");

    Ok(())
}

#[tokio::test]
async fn test_dbus_service_with_daemon_integration() -> Result<()> {
    use dots_family_daemon::config::DaemonConfig;
    use dots_family_daemon::daemon::Daemon;
    use dots_family_daemon::dbus_impl::FamilyDaemonService;
    use dots_family_daemon::monitoring_service::MonitoringService;
    use std::sync::Arc;

    let daemon = Arc::new(Daemon::new().await?);
    let config = DaemonConfig::load()?;
    let monitoring_service = MonitoringService::new();

    let _service =
        FamilyDaemonService::new_with_daemon(&config, monitoring_service, daemon).await?;

    Ok(())
}
