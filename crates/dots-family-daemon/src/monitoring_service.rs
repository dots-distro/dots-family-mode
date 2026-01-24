use std::{sync::Arc, time::SystemTime};

use anyhow::Result;
use dots_family_proto::events::ActivityEvent;
use serde_json::Value;
use tokio::{
    sync::Mutex,
    time::{interval, Duration},
};
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
    pub async fn new() -> Result<Self> {
        Ok(Self {
            process_monitor: Arc::new(Mutex::new(ProcessMonitorEbpf::new().await?)),
            network_monitor: Arc::new(Mutex::new(NetworkMonitorEbpf::new())),
            filesystem_monitor: Arc::new(Mutex::new(FilesystemMonitorEbpf::new())),
            running: Arc::new(Mutex::new(false)),
        })
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
            // Use compile-time path from build.rs if available (Nix build)
            // Otherwise fall back to runtime environment variable (local development)
            let network_path = if cfg!(feature = "nix-build")
                || option_env!("BPF_NETWORK_MONITOR_FILE").is_some()
            {
                option_env!("BPF_NETWORK_MONITOR_FILE").ok_or_else(|| {
                    anyhow::anyhow!("BPF_NETWORK_MONITOR_FILE not set at compile time")
                })?
            } else {
                &std::env::var("BPF_NETWORK_MONITOR_PATH").map_err(|_| {
                    anyhow::anyhow!(
                        "BPF_NETWORK_MONITOR_PATH not set - eBPF network monitoring required"
                    )
                })?
            };

            if let Err(e) = network_monitor.load(std::path::Path::new(&network_path)).await {
                return Err(anyhow::anyhow!(
                    "Failed to load eBPF network monitor from {}: {}",
                    network_path,
                    e
                ));
            }
        }

        {
            let mut filesystem_monitor = self.filesystem_monitor.lock().await;
            // Use compile-time path from build.rs if available (Nix build)
            // Otherwise fall back to runtime environment variable (local development)
            let filesystem_path = if cfg!(feature = "nix-build")
                || option_env!("BPF_FILESYSTEM_MONITOR_FILE").is_some()
            {
                option_env!("BPF_FILESYSTEM_MONITOR_FILE").ok_or_else(|| {
                    anyhow::anyhow!("BPF_FILESYSTEM_MONITOR_FILE not set at compile time")
                })?
            } else {
                &std::env::var("BPF_FILESYSTEM_MONITOR_PATH").map_err(|_| {
                    anyhow::anyhow!(
                        "BPF_FILESYSTEM_MONITOR_PATH not set - eBPF filesystem monitoring required"
                    )
                })?
            };

            if let Err(e) = filesystem_monitor.load(std::path::Path::new(&filesystem_path)).await {
                return Err(anyhow::anyhow!(
                    "Failed to load eBPF filesystem monitor from {}: {}",
                    filesystem_path,
                    e
                ));
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

    pub async fn get_recent_activities(&self) -> Result<Vec<ActivityEvent>> {
        let mut activities = Vec::new();

        let process_data = {
            let monitor = self.process_monitor.lock().await;
            monitor
                .collect_snapshot()
                .await
                .map_err(|e| anyhow::anyhow!("Process monitor error: {}", e))?
        };

        if let Some(recent_processes) = process_data["recent_processes"].as_array() {
            for process in recent_processes {
                if let (Some(pid), Some(executable), Some(args)) = (
                    process["pid"].as_u64(),
                    process["executable"].as_str(),
                    process["args"].as_array(),
                ) {
                    let args_vec: Vec<String> =
                        args.iter().filter_map(|arg| arg.as_str().map(|s| s.to_string())).collect();

                    activities.push(ActivityEvent::ProcessStarted {
                        pid: pid as u32,
                        executable: executable.to_string(),
                        args: args_vec,
                        timestamp: SystemTime::now(),
                    });
                }
            }
        }

        let network_data = {
            let monitor = self.network_monitor.lock().await;
            monitor
                .collect_snapshot()
                .await
                .map_err(|e| anyhow::anyhow!("Network monitor error: {}", e))?
        };

        if let Some(connections) = network_data["connections"].as_array() {
            for conn in connections {
                if let (Some(pid), Some(local_addr), Some(remote_addr)) = (
                    conn["pid"].as_u64(),
                    conn["local_addr"].as_str(),
                    conn["remote_addr"].as_str(),
                ) {
                    activities.push(ActivityEvent::NetworkConnection {
                        pid: pid as u32,
                        local_addr: local_addr.to_string(),
                        remote_addr: remote_addr.to_string(),
                        timestamp: SystemTime::now(),
                    });
                }
            }
        }

        Ok(activities)
    }

    #[allow(dead_code)]
    pub async fn health_check(&self) -> Result<bool> {
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
