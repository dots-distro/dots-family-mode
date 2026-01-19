use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use tracing::info;

/// Simple process monitor for testing (no eBPF dependency)
pub struct ProcessMonitorSimple {
    loaded: bool,
    stats: HashMap<String, u64>,
}

impl ProcessMonitorSimple {
    pub fn new() -> Self {
        Self { loaded: false, stats: HashMap::new() }
    }

    pub async fn load(&mut self) -> Result<()> {
        info!("Loading simple process monitor (no eBPF)");
        self.loaded = true;

        // Mock some basic process stats
        self.stats.insert("init".to_string(), 1);
        self.stats.insert("firefox".to_string(), 12345);

        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub async fn get_process_stats(&self, pid: u32) -> Result<Value, String> {
        if !self.loaded {
            return Err("Monitor not loaded".to_string());
        }

        match self.stats.get(&pid.to_string()) {
            Some(stats) => Ok(serde_json::json!({"pid": pid, "stats": stats})),
            None => Err(format!("No stats for PID {}", pid)),
        }
    }
}

/// Fallback manager when eBPF is not available
pub struct FallbackManager {
    loaded_monitors: HashMap<String, ProcessMonitorSimple>,
}

impl FallbackManager {
    pub fn new() -> Self {
        Self { loaded_monitors: HashMap::new() }
    }

    pub async fn load_monitor(
        &mut self,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Loading fallback monitor: {}", name);
        let mut monitor = ProcessMonitorSimple::new();
        monitor.load().await?;
        self.loaded_monitors.insert(name.to_string(), monitor);
        Ok(())
    }

    pub async fn get_stats(&self, name: &str, pid: u32) -> Result<Value, String> {
        match self.loaded_monitors.get(name) {
            Some(monitor) => monitor.get_process_stats(pid).await,
            None => Err(format!("Monitor '{}' not loaded", name)),
        }
    }
}
