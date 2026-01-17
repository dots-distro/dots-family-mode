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
    let manager = EbpfManager::new().await?;
    let status = manager.get_health_status().await;

    assert_eq!(status.programs_loaded, 0);
    assert!(!status.all_healthy);
    assert_eq!(status.program_status.len(), 3);

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
