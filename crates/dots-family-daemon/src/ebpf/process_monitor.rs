use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ProcessEvent {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub gid: u32,
    pub comm: [u8; 16],
    pub filename: [u8; 256],
    pub event_type: u32,
    pub timestamp: u64,
}

#[derive(Default)]
struct ProcessStats {
    total_events: u64,
    exec_events: u64,
    exit_events: u64,
}

pub struct ProcessMonitorEbpf {
    stats: Arc<Mutex<ProcessStats>>,
    loaded: bool,
}

impl ProcessMonitorEbpf {
    pub fn new() -> Self {
        Self { stats: Arc::new(Mutex::new(ProcessStats::default())), loaded: false }
    }

    pub async fn load(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Attempting to load eBPF process monitor");

        if !super::ebpf_available() {
            return Err("eBPF not available: insufficient permissions or kernel support".into());
        }

        warn!("eBPF loading not yet implemented, using fallback simulation mode");
        self.loaded = false;
        Ok(())
    }

    pub async fn collect_snapshot(
        &self,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let stats_guard = self.stats.lock().await;

        Ok(json!({
            "ebpf_loaded": self.loaded,
            "collection_method": if self.loaded { "ebpf" } else { "procfs_fallback" },
            "total_events": stats_guard.total_events,
            "exec_events": stats_guard.exec_events,
            "exit_events": stats_guard.exit_events,
            "recent_processes": self.get_recent_processes_procfs().await?
        }))
    }

    async fn get_recent_processes_procfs(
        &self,
    ) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut processes = Vec::new();

        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if let Ok(pid) = file_name.parse::<u32>() {
                        if let Ok(proc_info) = self.read_proc_info(pid).await {
                            processes.push(proc_info);
                        }
                    }
                }
            }
        }

        processes.sort_by(|a, b| {
            b["start_time"].as_u64().unwrap_or(0).cmp(&a["start_time"].as_u64().unwrap_or(0))
        });

        Ok(processes.into_iter().take(50).collect())
    }

    async fn read_proc_info(
        &self,
        pid: u32,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let stat_path = format!("/proc/{}/stat", pid);
        let stat_content = std::fs::read_to_string(&stat_path)?;

        let cmdline_path = format!("/proc/{}/cmdline", pid);
        let cmdline = std::fs::read_to_string(&cmdline_path)
            .unwrap_or_default()
            .replace('\0', " ")
            .trim()
            .to_string();

        let parts: Vec<&str> = stat_content.split_whitespace().collect();
        if parts.len() < 22 {
            return Err("Invalid stat format".into());
        }

        let comm = parts[1].trim_start_matches('(').trim_end_matches(')');
        let ppid: u32 = parts[3].parse()?;
        let start_time: u64 = parts[21].parse()?;

        Ok(json!({
            "pid": pid,
            "ppid": ppid,
            "comm": comm,
            "cmdline": cmdline,
            "start_time": start_time,
            "source": "procfs"
        }))
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

impl Default for ProcessMonitorEbpf {
    fn default() -> Self {
        Self::new()
    }
}
