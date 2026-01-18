use dots_family_daemon::ebpf::network_monitor::NetworkMonitorEbpf;
use std::time::Duration;

#[tokio::test]
async fn test_network_monitor_loads_successfully() {
    let mut monitor = NetworkMonitorEbpf::new();

    let result = monitor.load("eth0").await;
    assert!(result.is_ok(), "NetworkMonitorEbpf should load successfully");
    assert!(monitor.is_loaded(), "NetworkMonitorEbpf should report as loaded");
}

#[tokio::test]
async fn test_network_monitor_event_collection() {
    let mut monitor = NetworkMonitorEbpf::new();
    monitor.load("eth0").await.expect("Failed to load network monitor");

    let mut receiver = monitor.start_collection().await.expect("Failed to start event collection");

    let event = tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await;
    assert!(event.is_ok(), "Should receive network event within timeout");
    assert!(event.unwrap().is_some(), "Should receive actual event data");
}

#[tokio::test]
async fn test_network_monitor_collects_multiple_events() {
    let mut monitor = NetworkMonitorEbpf::new();
    monitor.load("eth0").await.expect("Failed to load network monitor");

    let mut receiver = monitor.start_collection().await.expect("Failed to start event collection");

    let mut event_count = 0;
    let start_time = std::time::Instant::now();

    while event_count < 3 && start_time.elapsed() < Duration::from_secs(10) {
        if let Ok(Some(_event)) =
            tokio::time::timeout(Duration::from_millis(500), receiver.recv()).await
        {
            event_count += 1;
        }
    }

    assert!(event_count >= 3, "Should collect multiple network events, got: {}", event_count);
}

#[tokio::test]
async fn test_network_monitor_basic_functionality() {
    let mut monitor = NetworkMonitorEbpf::new();
    monitor.load("eth0").await.expect("Failed to load network monitor");

    let mut receiver = monitor.start_collection().await.expect("Failed to start event collection");

    let event = tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await;
    assert!(event.is_ok(), "Should receive network event within timeout");
}

#[tokio::test]
async fn test_network_monitor_snapshot_collection() {
    let mut monitor = NetworkMonitorEbpf::new();
    monitor.load("eth0").await.expect("Failed to load network monitor");

    let snapshot = monitor.collect_snapshot().await.expect("Failed to collect snapshot");

    assert!(snapshot.is_object(), "Snapshot should be JSON object");
    assert!(snapshot["ebpf_loaded"].as_bool().unwrap_or(false), "Should report eBPF as loaded");
    assert!(snapshot["connections"].is_array(), "Should include connections");
}

#[tokio::test]
async fn test_network_monitor_event_format() {
    let mut monitor = NetworkMonitorEbpf::new();
    monitor.load("eth0").await.expect("Failed to load network monitor");

    let mut receiver = monitor.start_collection().await.expect("Failed to start event collection");

    if let Ok(Some(event)) = tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await {
        assert!(event["event_type"].is_string(), "Event should have event_type field");
        assert!(event["timestamp"].is_number(), "Event should have timestamp field");
        assert!(event["pid"].is_number(), "Event should have pid field");
        assert!(event["src_addr"].is_string(), "Event should have src_addr field");
        assert!(event["dst_addr"].is_string(), "Event should have dst_addr field");
        assert!(event["src_port"].is_number(), "Event should have src_port field");
        assert!(event["dst_port"].is_number(), "Event should have dst_port field");
        assert!(event["protocol"].is_string(), "Event should have protocol field");
        assert!(event["source"].is_string(), "Event should have source field");

        println!("Sample network event: {}", serde_json::to_string_pretty(&event).unwrap());
    } else {
        panic!("Should receive at least one network event for format validation");
    }
}
