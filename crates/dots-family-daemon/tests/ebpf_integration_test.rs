use anyhow::Result;
use dots_family_daemon::ebpf::EbpfManager;

#[tokio::test]
async fn test_ebpf_manager_initialization() -> Result<()> {
    // Test that EbpfManager can be created without panicking
    let manager = EbpfManager::new().await?;

    // Should be able to get initial health status
    let health = manager.get_health_status().await;

    // Health status should have entries for all three program types
    assert!(health.program_status.contains_key("process_monitor"));
    assert!(health.program_status.contains_key("network_monitor"));
    assert!(health.program_status.contains_key("filesystem_monitor"));

    Ok(())
}

#[tokio::test]
async fn test_ebpf_programs_loading() -> Result<()> {
    let mut manager = EbpfManager::new().await?;

    // This should not fail even if eBPF programs are not available
    // (graceful error handling)
    manager.load_all_programs().await?;

    let health = manager.get_health_status().await;

    // Should have attempted to load all programs
    assert_eq!(health.program_status.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_ebpf_health_check() -> Result<()> {
    let manager = EbpfManager::new().await?;
    let status = manager.get_health_status().await;
    assert_eq!(status.programs_loaded, 3);
    assert!(status.all_healthy);

    Ok(())
}
