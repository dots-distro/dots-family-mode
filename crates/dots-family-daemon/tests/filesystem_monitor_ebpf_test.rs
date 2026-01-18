use dots_family_daemon::ebpf::filesystem_monitor::FilesystemMonitorEbpf;
use std::time::Duration;

#[tokio::test]
async fn test_filesystem_monitor_loads_successfully() {
    let mut monitor = FilesystemMonitorEbpf::new();

    let result = monitor.load().await;
    assert!(result.is_ok(), "FilesystemMonitorEbpf should load successfully");
    assert!(monitor.is_loaded(), "FilesystemMonitorEbpf should report as loaded");
}

#[tokio::test]
async fn test_filesystem_monitor_event_collection() {
    let mut monitor = FilesystemMonitorEbpf::new();
    monitor.load().await.expect("Failed to load filesystem monitor");

    let mut receiver = monitor.start_collection().await.expect("Failed to start event collection");

    let event = tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await;
    assert!(event.is_ok(), "Should receive filesystem event within timeout");
    assert!(event.unwrap().is_some(), "Should receive actual event data");
}

#[tokio::test]
async fn test_filesystem_monitor_collects_multiple_events() {
    let mut monitor = FilesystemMonitorEbpf::new();
    monitor.load().await.expect("Failed to load filesystem monitor");

    let mut receiver = monitor.start_collection().await.expect("Failed to start event collection");

    let mut event_count = 0;
    let start_time = std::time::Instant::now();

    while event_count < 3 && start_time.elapsed() < Duration::from_secs(10) {
        if let Ok(Some(_event)) =
            tokio::time::timeout(Duration::from_millis(800), receiver.recv()).await
        {
            event_count += 1;
        }
    }

    assert!(event_count >= 3, "Should collect multiple filesystem events, got: {}", event_count);
}

#[tokio::test]
async fn test_filesystem_monitor_path_filtering() {
    let mut monitor = FilesystemMonitorEbpf::new();
    monitor.load().await.expect("Failed to load filesystem monitor");

    let test_filters = vec!["/home".to_string(), "/tmp".to_string()];
    monitor.set_path_filters(test_filters.clone());

    let mut receiver = monitor.start_collection().await.expect("Failed to start event collection");

    let event = tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await;
    assert!(event.is_ok(), "Should receive filtered filesystem event");
}

#[tokio::test]
async fn test_filesystem_monitor_basic_functionality() {
    let mut monitor = FilesystemMonitorEbpf::new();
    monitor.load().await.expect("Failed to load filesystem monitor");

    let mut receiver = monitor.start_collection().await.expect("Failed to start event collection");

    let event = tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await;
    assert!(event.is_ok(), "Should receive filesystem event within timeout");
}

#[tokio::test]
async fn test_filesystem_monitor_snapshot_collection() {
    let mut monitor = FilesystemMonitorEbpf::new();
    monitor.load().await.expect("Failed to load filesystem monitor");

    let snapshot = monitor.collect_snapshot().await.expect("Failed to collect snapshot");

    assert!(snapshot.is_object(), "Snapshot should be JSON object");
    assert!(snapshot["ebpf_loaded"].as_bool().unwrap_or(false), "Should report eBPF as loaded");
    assert!(snapshot["collection_method"].is_string(), "Should report collection method");
}

#[tokio::test]
async fn test_filesystem_monitor_event_format() {
    let mut monitor = FilesystemMonitorEbpf::new();
    monitor.load().await.expect("Failed to load filesystem monitor");

    let mut receiver = monitor.start_collection().await.expect("Failed to start event collection");

    if let Ok(Some(event)) = tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await {
        assert!(event["event_type"].is_string(), "Event should have event_type field");
        assert!(event["timestamp"].is_number(), "Event should have timestamp field");
        assert!(event["pid"].is_number(), "Event should have pid field");
        assert!(event["filename"].is_string(), "Event should have filename field");
        assert!(event["source"].is_string(), "Event should have source field");

        println!("Sample filesystem event: {}", serde_json::to_string_pretty(&event).unwrap());
    } else {
        panic!("Should receive at least one filesystem event for format validation");
    }
}
