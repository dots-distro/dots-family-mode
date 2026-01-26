use std::{sync::Arc, time::SystemTime};

use anyhow::Result;
use dots_family_proto::events::ActivityEvent;
use serde_json::Value;
use tokio::{
    sync::Mutex,
    time::{interval, Duration},
};
use tracing::{error, info, warn};

use crate::ebpf::{
    DiskIoMonitorEbpf, FilesystemMonitorEbpf, MemoryMonitorEbpf, NetworkMonitorEbpf,
    ProcessMonitorEbpf,
};

#[derive(Clone)]
pub struct MonitoringService {
    process_monitor: Arc<Mutex<ProcessMonitorEbpf>>,
    network_monitor: Arc<Mutex<NetworkMonitorEbpf>>,
    filesystem_monitor: Arc<Mutex<FilesystemMonitorEbpf>>,
    memory_monitor: Arc<Mutex<MemoryMonitorEbpf>>,
    disk_io_monitor: Arc<Mutex<DiskIoMonitorEbpf>>,
    running: Arc<Mutex<bool>>,
}

impl MonitoringService {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            process_monitor: Arc::new(Mutex::new(ProcessMonitorEbpf::new().await?)),
            network_monitor: Arc::new(Mutex::new(NetworkMonitorEbpf::new())),
            filesystem_monitor: Arc::new(Mutex::new(FilesystemMonitorEbpf::new())),
            memory_monitor: Arc::new(Mutex::new(MemoryMonitorEbpf::new()?)),
            disk_io_monitor: Arc::new(Mutex::new(DiskIoMonitorEbpf::new()?)),
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

        {
            let mut memory_monitor = self.memory_monitor.lock().await;
            let memory_path = if cfg!(feature = "nix-build")
                || option_env!("BPF_MEMORY_MONITOR_FILE").is_some()
            {
                option_env!("BPF_MEMORY_MONITOR_FILE").ok_or_else(|| {
                    anyhow::anyhow!("BPF_MEMORY_MONITOR_FILE not set at compile time")
                })?
            } else {
                &std::env::var("BPF_MEMORY_MONITOR_PATH").unwrap_or_default()
            };

            if !memory_path.is_empty() {
                if let Err(e) = memory_monitor.load(std::path::Path::new(&memory_path)).await {
                    warn!("Failed to load eBPF memory monitor: {}, continuing without it", e);
                }
            } else {
                warn!("BPF_MEMORY_MONITOR_PATH not set, continuing without memory monitoring");
            }
        }

        {
            let mut disk_io_monitor = self.disk_io_monitor.lock().await;
            let disk_io_path = if cfg!(feature = "nix-build")
                || option_env!("BPF_DISK_IO_MONITOR_FILE").is_some()
            {
                option_env!("BPF_DISK_IO_MONITOR_FILE").ok_or_else(|| {
                    anyhow::anyhow!("BPF_DISK_IO_MONITOR_FILE not set at compile time")
                })?
            } else {
                &std::env::var("BPF_DISK_IO_MONITOR_PATH").unwrap_or_default()
            };

            if !disk_io_path.is_empty() {
                if let Err(e) = disk_io_monitor.load(std::path::Path::new(&disk_io_path)).await {
                    warn!("Failed to load eBPF disk I/O monitor: {}, continuing without it", e);
                }
            } else {
                warn!("BPF_DISK_IO_MONITOR_PATH not set, continuing without disk I/O monitoring");
            }
        }

        let process_monitor_clone = Arc::clone(&self.process_monitor);
        let network_monitor_clone = Arc::clone(&self.network_monitor);
        let filesystem_monitor_clone = Arc::clone(&self.filesystem_monitor);
        let memory_monitor_clone = Arc::clone(&self.memory_monitor);
        let disk_io_monitor_clone = Arc::clone(&self.disk_io_monitor);
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
                    &memory_monitor_clone,
                    &disk_io_monitor_clone,
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

        let memory_data = {
            let monitor = self.memory_monitor.lock().await;
            monitor
                .collect_snapshot()
                .await
                .map_err(|e| anyhow::anyhow!("Memory monitor error: {}", e))?
        };

        let disk_io_data = {
            let monitor = self.disk_io_monitor.lock().await;
            monitor
                .collect_snapshot()
                .await
                .map_err(|e| anyhow::anyhow!("Disk I/O monitor error: {}", e))?
        };

        Ok(serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp(),
            "process_monitoring": process_data,
            "network_monitoring": network_data,
            "filesystem_monitoring": filesystem_data,
            "memory_monitoring": memory_data,
            "disk_io_monitoring": disk_io_data
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

        let memory_healthy = {
            let monitor = self.memory_monitor.lock().await;
            monitor.collect_snapshot().await.is_ok()
        };

        let disk_io_healthy = {
            let monitor = self.disk_io_monitor.lock().await;
            monitor.collect_snapshot().await.is_ok()
        };

        Ok(process_healthy
            && network_healthy
            && filesystem_healthy
            && memory_healthy
            && disk_io_healthy)
    }
}

async fn collect_monitoring_data(
    process_monitor: &Arc<Mutex<ProcessMonitorEbpf>>,
    network_monitor: &Arc<Mutex<NetworkMonitorEbpf>>,
    filesystem_monitor: &Arc<Mutex<FilesystemMonitorEbpf>>,
    memory_monitor: &Arc<Mutex<MemoryMonitorEbpf>>,
    disk_io_monitor: &Arc<Mutex<DiskIoMonitorEbpf>>,
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

    let memory_snapshot = {
        let monitor = memory_monitor.lock().await;
        monitor
            .collect_snapshot()
            .await
            .map_err(|e| anyhow::anyhow!("Memory monitor error: {}", e))?
    };

    let disk_io_snapshot = {
        let monitor = disk_io_monitor.lock().await;
        monitor
            .collect_snapshot()
            .await
            .map_err(|e| anyhow::anyhow!("Disk I/O monitor error: {}", e))?
    };

    info!(
        "Collected monitoring data - processes: {}, connections: {}, files: {}, memory events: {}, disk I/O events: {}",
        process_snapshot["recent_processes"].as_array().map(|v| v.len()).unwrap_or(0),
        network_snapshot["connections"].as_array().map(|v| v.len()).unwrap_or(0),
        filesystem_snapshot["recent_file_access"].as_array().map(|v| v.len()).unwrap_or(0),
        memory_snapshot["recent_events"].as_array().map(|v| v.len()).unwrap_or(0),
        disk_io_snapshot["recent_events"].as_array().map(|v| v.len()).unwrap_or(0)
    );

    Ok(())
}
