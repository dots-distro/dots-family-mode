use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::ebpf::{FilesystemMonitorEbpf, NetworkMonitorEbpf, ProcessMonitorEbpf};

#[derive(Clone)]
pub struct MonitoringService {
    process_monitor: Arc<Mutex<ProcessMonitorEbpf>>,
    network_monitor: Arc<Mutex<NetworkMonitorEbpf>>,
    filesystem_monitor: Arc<Mutex<FilesystemMonitorEbpf>>,
    running: Arc<Mutex<bool>>,
}

impl MonitoringService {
    pub fn new() -> Self {
        Self {
            process_monitor: Arc::new(Mutex::new(ProcessMonitorEbpf::new())),
            network_monitor: Arc::new(Mutex::new(NetworkMonitorEbpf::new())),
            filesystem_monitor: Arc::new(Mutex::new(FilesystemMonitorEbpf::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting monitoring service");

        let mut running = self.running.lock().await;
        if *running {
            return Ok(());
        }
        *running = true;

        {
            let mut process_monitor = self.process_monitor.lock().await;
            if let Err(e) = process_monitor.load().await {
                warn!("Failed to load eBPF process monitor: {}, using fallback", e);
            }
        }

        {
            let mut network_monitor = self.network_monitor.lock().await;
            if let Err(e) = network_monitor.load("eth0").await {
                warn!("Failed to load eBPF network monitor: {}, using fallback", e);
            }
        }

        {
            let mut filesystem_monitor = self.filesystem_monitor.lock().await;
            if let Err(e) = filesystem_monitor.load().await {
                warn!("Failed to load eBPF filesystem monitor: {}, using fallback", e);
            }
        }

        let process_monitor_clone = Arc::clone(&self.process_monitor);
        let network_monitor_clone = Arc::clone(&self.network_monitor);
        let filesystem_monitor_clone = Arc::clone(&self.filesystem_monitor);
        let running_clone = Arc::clone(&self.running);

        tokio::spawn(async move {
            let mut interval_timer = interval(Duration::from_secs(10));

            loop {
                interval_timer.tick().await;

                {
                    let running = running_clone.lock().await;
                    if !*running {
                        break;
                    }
                }

                if let Err(e) = collect_monitoring_data(
                    &process_monitor_clone,
                    &network_monitor_clone,
                    &filesystem_monitor_clone,
                )
                .await
                {
                    error!("Failed to collect monitoring data: {}", e);
                }
            }

            info!("Monitoring service collection loop stopped");
        });

        info!("Monitoring service started successfully");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        info!("Stopping monitoring service");
        let mut running = self.running.lock().await;
        *running = false;
        info!("Monitoring service stopped");
        Ok(())
    }

    pub async fn get_monitoring_snapshot(&self) -> Result<Value> {
        let process_data = {
            let monitor = self.process_monitor.lock().await;
            monitor
                .collect_snapshot()
                .await
                .map_err(|e| anyhow::anyhow!("Process monitor error: {}", e))?
        };

        let network_data = {
            let monitor = self.network_monitor.lock().await;
            monitor
                .collect_snapshot()
                .await
                .map_err(|e| anyhow::anyhow!("Network monitor error: {}", e))?
        };

        let filesystem_data = {
            let monitor = self.filesystem_monitor.lock().await;
            monitor
                .collect_snapshot()
                .await
                .map_err(|e| anyhow::anyhow!("Filesystem monitor error: {}", e))?
        };

        Ok(serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp(),
            "process_monitoring": process_data,
            "network_monitoring": network_data,
            "filesystem_monitoring": filesystem_data
        }))
    }

    pub async fn health_check(&self) -> Result<bool> {
        // Simple health check - verify all monitors are accessible
        let process_healthy = {
            let monitor = self.process_monitor.lock().await;
            monitor.collect_snapshot().await.is_ok()
        };

        let network_healthy = {
            let monitor = self.network_monitor.lock().await;
            monitor.collect_snapshot().await.is_ok()
        };

        let filesystem_healthy = {
            let monitor = self.filesystem_monitor.lock().await;
            monitor.collect_snapshot().await.is_ok()
        };

        Ok(process_healthy && network_healthy && filesystem_healthy)
    }
}

async fn collect_monitoring_data(
    process_monitor: &Arc<Mutex<ProcessMonitorEbpf>>,
    network_monitor: &Arc<Mutex<NetworkMonitorEbpf>>,
    filesystem_monitor: &Arc<Mutex<FilesystemMonitorEbpf>>,
) -> Result<()> {
    let process_snapshot = {
        let monitor = process_monitor.lock().await;
        monitor
            .collect_snapshot()
            .await
            .map_err(|e| anyhow::anyhow!("Process monitor error: {}", e))?
    };

    let network_snapshot = {
        let monitor = network_monitor.lock().await;
        monitor
            .collect_snapshot()
            .await
            .map_err(|e| anyhow::anyhow!("Network monitor error: {}", e))?
    };

    let filesystem_snapshot = {
        let monitor = filesystem_monitor.lock().await;
        monitor
            .collect_snapshot()
            .await
            .map_err(|e| anyhow::anyhow!("Filesystem monitor error: {}", e))?
    };

    info!(
        "Collected monitoring data - processes: {}, connections: {}, files: {}",
        process_snapshot["recent_processes"].as_array().map(|v| v.len()).unwrap_or(0),
        network_snapshot["connections"].as_array().map(|v| v.len()).unwrap_or(0),
        filesystem_snapshot["recent_file_access"].as_array().map(|v| v.len()).unwrap_or(0)
    );

    Ok(())
}
