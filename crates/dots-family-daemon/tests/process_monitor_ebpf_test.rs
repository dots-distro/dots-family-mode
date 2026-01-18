use dots_family_daemon::ebpf::ProcessMonitorEbpf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_process_monitor_loads_successfully() {
    let mut monitor = ProcessMonitorEbpf::new();
    let result = monitor.load().await;
    assert!(result.is_ok(), "eBPF process monitor should load successfully");
    assert!(monitor.is_loaded(), "Monitor should be loaded");
}

#[tokio::test]
async fn test_process_monitor_can_collect_snapshot() {
    let mut monitor = ProcessMonitorEbpf::new();
    monitor.load().await.unwrap();

    let snapshot = monitor.collect_snapshot().await;
    assert!(snapshot.is_ok(), "Should be able to collect process snapshot");

    let snapshot_data = snapshot.unwrap();
    assert!(snapshot_data.is_object(), "Snapshot should be a JSON object");
    assert!(snapshot_data["ebpf_loaded"].is_boolean(), "Should report ebpf_loaded status");
    assert!(snapshot_data["collection_method"].is_string(), "Should report collection method");
    assert!(snapshot_data["recent_processes"].is_array(), "Should include recent processes array");
}

#[tokio::test]
async fn test_process_monitor_snapshot_contains_process_data() {
    let mut monitor = ProcessMonitorEbpf::new();
    monitor.load().await.unwrap();

    let snapshot = monitor.collect_snapshot().await.unwrap();
    let processes = &snapshot["recent_processes"];

    if let Some(process_array) = processes.as_array() {
        if !process_array.is_empty() {
            let first_process = &process_array[0];
            assert!(first_process["pid"].is_number(), "Process should have PID");
            assert!(first_process["ppid"].is_number(), "Process should have PPID");
            assert!(first_process["comm"].is_string(), "Process should have command name");
            assert!(first_process["source"].is_string(), "Process should have data source");
        }
    }
}

#[tokio::test]
async fn test_process_monitor_basic_functionality() {
    let mut monitor = ProcessMonitorEbpf::new();
    monitor.load().await.expect("Failed to load process monitor");

    let snapshot = monitor.collect_snapshot().await.expect("Failed to collect snapshot");
    assert!(snapshot["ebpf_loaded"].as_bool().unwrap_or(false), "Should report as loaded");

    let collection_method = snapshot["collection_method"].as_str().unwrap_or("");
    assert!(
        collection_method.contains("ebpf") || collection_method.contains("procfs"),
        "Should use either eBPF or procfs collection method"
    );
}

#[tokio::test]
async fn test_process_monitor_fallback_mode() {
    let mut monitor = ProcessMonitorEbpf::new();
    monitor.load().await.unwrap();

    let snapshot = monitor.collect_snapshot().await.unwrap();

    let collection_method = snapshot["collection_method"].as_str().unwrap_or("");
    if collection_method.contains("procfs") {
        let processes = snapshot["recent_processes"].as_array().unwrap();
        if !processes.is_empty() {
            let process = &processes[0];
            assert!(
                process["source"].as_str().unwrap_or("").contains("procfs"),
                "Fallback mode should use procfs source"
            );
        }
    }
}

#[tokio::test]
async fn test_process_monitor_multiple_snapshots() {
    let mut monitor = ProcessMonitorEbpf::new();
    monitor.load().await.unwrap();

    let snapshot1 = monitor.collect_snapshot().await.unwrap();
    sleep(Duration::from_millis(100)).await;
    let snapshot2 = monitor.collect_snapshot().await.unwrap();

    assert!(
        snapshot1.is_object() && snapshot2.is_object(),
        "Both snapshots should be valid objects"
    );
    assert_eq!(
        snapshot1["ebpf_loaded"], snapshot2["ebpf_loaded"],
        "eBPF loaded status should be consistent"
    );
}
