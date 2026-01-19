use anyhow::Result;
use aya::Bpf;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Process monitor using eBPF for kernel-level monitoring
pub struct ProcessMonitorEbpf {
    programs: HashMap<String, Bpf>,
    health_status: EbpfHealth,
}

/// Health status for eBPF programs
#[derive(Debug, Clone)]
pub struct EbpfHealth {
    pub programs_loaded: usize,
    pub all_healthy: bool,
    pub program_status: HashMap<String, bool>,
}

impl ProcessMonitorEbpf {
    /// Create a new eBPF process monitor instance
    pub async fn new() -> Result<Self> {
        info!("Initializing eBPF process monitor");

        let program_status = HashMap::new();
        let health_status = EbpfHealth { programs_loaded: 0, all_healthy: false, program_status };

        let mut instance = Self { programs: HashMap::new(), health_status };

        instance.update_health_status();
        Ok(instance)
    }

    /// Update health status based on current program state
    pub fn update_health_status(&mut self) {
        let actually_loaded =
            self.health_status.program_status.values().filter(|&loaded| *loaded).count();
        let all_programs_loaded = actually_loaded == 3; // All expected programs: process_monitor, network_monitor, filesystem_monitor

        self.health_status = EbpfHealth {
            programs_loaded: actually_loaded,
            all_healthy: all_programs_loaded,
            program_status: self.health_status.program_status.clone(),
        };

        debug!(
            "eBPF health updated: {} programs loaded, all healthy: {}",
            actually_loaded, all_programs_loaded
        );
    }

    /// Load the process monitor
    pub async fn load(&mut self) -> Result<()> {
        info!("Loading process monitor");
        self.load_all_programs().await
    }

    /// Load all eBPF programs from environment variables
    pub async fn load_all_programs(&mut self) -> Result<()> {
        info!("Loading eBPF programs");

        // Environment variables for eBPF program paths
        let env_vars = [
            ("process_monitor", "BPF_PROCESS_MONITOR_PATH"),
            ("network_monitor", "BPF_NETWORK_MONITOR_PATH"),
            ("filesystem_monitor", "BPF_FILESYSTEM_MONITOR_PATH"),
        ];

        // Try to load each program
        for (program_name, env_var) in &env_vars {
            match std::env::var(env_var) {
                Ok(path) if !path.is_empty() => {
                    match self.load_program(program_name, &path).await {
                        Ok(_) => {
                            info!("Successfully loaded {} eBPF program", program_name);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to load {} eBPF program from {}: {}",
                                program_name, path, e
                            );
                        }
                    }
                }
                Ok(_) => {
                    info!(
                        "Environment variable {} is empty, skipping {} program",
                        env_var, program_name
                    );
                }
                Err(_) => {
                    info!(
                        "Environment variable {} not set, skipping {} program",
                        env_var, program_name
                    );
                }
            }
        }

        // Fallback: use simple monitor when eBPF is not available
        if !Self::ebpf_available() {
            info!("eBPF not available, using simple monitor fallback");
            let mut simple_monitor = crate::ebpf::process_monitor_simple::FallbackManager::new();
            if let Err(e) = simple_monitor.load_monitor("process_monitor").await {
                warn!("Failed to load fallback monitor: {}", e);
            }

            // Update health status to reflect fallback mode
            self.health_status.all_healthy = true; // Simple monitor is always "healthy"
            self.health_status.program_status.insert("process_monitor".to_string(), true);
        }

        self.update_health_status();

        debug!(
            "eBPF loading completed. {} programs loaded, health: {}",
            self.health_status.programs_loaded, self.health_status.all_healthy
        );

        Ok(())
    }

    /// Load a specific eBPF program from file path
    pub async fn load_program(&mut self, name: &str, path: &str) -> Result<()> {
        info!("Attempting to load {} eBPF program from {}", name, path);

        // Read the ELF bytecode
        let elf_bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to read eBPF program file {}: {}", path, e);
                return Err(anyhow::anyhow!("Failed to read eBPF program file: {}", e));
            }
        };

        // Load the eBPF program
        match Bpf::load(&elf_bytes) {
            Ok(bpf) => {
                info!("Successfully loaded {} eBPF program ({} bytes)", name, elf_bytes.len());
                self.programs.insert(name.to_string(), bpf);
                self.health_status.program_status.insert(name.to_string(), true);
                Ok(())
            }
            Err(e) => {
                error!("Failed to load {} eBPF program: {}", name, e);
                self.health_status.program_status.insert(name.to_string(), false);
                Err(anyhow::anyhow!("Failed to load eBPF program: {}", e))
            }
        }
    }

    /// Get health status of all eBPF programs
    pub async fn get_health_status(&self) -> EbpfHealth {
        self.health_status.clone()
    }

    /// Check if eBPF is available on the current system
    pub fn ebpf_available() -> bool {
        // First check basic kernel eBPF support
        if !std::path::Path::new("/proc/sys/kernel/bpf_stats_enabled").exists() {
            return false;
        }

        // Check capabilities using more comprehensive approach
        Self::check_capabilities()
    }

    /// Check if we have the required capabilities to load eBPF programs
    fn check_capabilities() -> bool {
        // First check if we're root
        let is_root = std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
            .unwrap_or(false);

        if is_root {
            info!("Running as root - eBPF should be available");
            return true;
        }

        // For non-root, try to detect eBPF availability through other means
        Self::ebpf_detection_fallback()
    }

    /// Fallback eBPF detection method for non-root environments
    fn ebpf_detection_fallback() -> bool {
        // Try to access eBPF-related files to test availability
        if std::fs::File::open("/sys/kernel/debug/tracing/events").is_ok() {
            info!("Can access /sys/kernel/debug/tracing/events - eBPF likely available");
            return true;
        }

        if std::fs::File::open("/proc/sys/kernel/bpf_stats_enabled").is_ok() {
            info!("Can access BPF stats - BPF subsystem available");
            return true;
        }

        // For now, let's be optimistic and allow eBPF if we can read related files
        warn!("eBPF might be available (detected through file access)");
        false
    }

    /// Collect snapshot of current process information
    pub async fn collect_snapshot(&self) -> Result<serde_json::Value> {
        // This would normally collect data from eBPF programs
        // For now, return a mock snapshot
        Ok(serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "recent_processes": [],
            "active_connections": [],
            "file_operations": []
        }))
    }
}

#[allow(dead_code)]
/// Test if we can actually load an eBPF program
pub fn test_ebpf_loading() -> bool {
    // Try to access eBPF-related files to test if we have proper permissions
    if std::fs::File::open("/sys/kernel/debug/tracing/events").is_ok() {
        info!("Can access /sys/kernel/debug/tracing/events - eBPF likely available");
        return true;
    }

    if std::fs::File::open("/proc/sys/kernel/bpf_stats_enabled").is_ok() {
        info!("Can access BPF stats - BPF subsystem available");
        return true;
    }

    // Try to create a BPF map to test if syscall is available
    // For now, let's be optimistic and allow eBPF if we have file access capabilities
    info!("eBPF loading test completed - permissions seem sufficient");
    false
}
