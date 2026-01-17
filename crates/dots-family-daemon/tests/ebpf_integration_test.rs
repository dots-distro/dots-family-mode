use anyhow::Result;
use dots_family_daemon::ebpf::EbpfManager;

#[tokio::test]
async fn test_ebpf_manager_initialization() -> Result<()> {
    // Test that EbpfManager can be created without panicking
    let manager = EbpfManager::new().await?;

    // Should be able to get initial health status
    let health = manager.get_health_status();

    // Health status should have entries for all three program types
    assert!(health.programs.contains_key("process"));
    assert!(health.programs.contains_key("network"));
    assert!(health.programs.contains_key("filesystem"));

    Ok(())
}

#[tokio::test]
async fn test_ebpf_programs_loading() -> Result<()> {
    let mut manager = EbpfManager::new().await?;

    // This should not fail even if eBPF programs are not available
    // (graceful error handling)
    manager.load_all_programs().await?;

    let health = manager.get_health_status();

    // Should have attempted to load all programs
    assert_eq!(health.programs.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_ebpf_health_check() -> Result<()> {
    let manager = EbpfManager::new().await?;
    let health = manager.get_health_status();

    // Health should have the expected structure
    assert!(health.programs.contains_key("process"));
    assert!(health.programs.contains_key("network"));
    assert!(health.programs.contains_key("filesystem"));

    // Each program should have a status
    for (name, _status) in &health.programs {
        assert!(!name.is_empty());
        // Status should be either true (loaded) or false (failed/not loaded)
        // Don't assert specific values since eBPF programs may not be available in test env
    }

    Ok(())
}
