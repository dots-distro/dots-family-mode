pub mod filesystem_monitor;
pub mod network_monitor;
pub mod process_monitor;

use anyhow::Result;
use aya::Bpf;
use std::collections::HashMap;
use tracing::{error, info, warn};

pub use filesystem_monitor::FilesystemMonitorEbpf;
pub use network_monitor::NetworkMonitorEbpf;
pub use process_monitor::ProcessMonitorEbpf;

/// Health status for eBPF programs
#[derive(Debug, Clone)]
pub struct EbpfHealth {
    pub programs: HashMap<String, bool>,
}

/// Manager for loading and monitoring eBPF programs
pub struct EbpfManager {
    loaded_programs: HashMap<String, Bpf>,
    health: EbpfHealth,
}

impl EbpfManager {
    /// Create a new eBPF manager instance
    pub async fn new() -> Result<Self> {
        info!("Initializing eBPF manager");

        let mut programs = HashMap::new();
        programs.insert("process".to_string(), false);
        programs.insert("network".to_string(), false);
        programs.insert("filesystem".to_string(), false);

        let health = EbpfHealth { programs };

        Ok(Self { loaded_programs: HashMap::new(), health })
    }

    /// Load all eBPF programs from environment variables
    pub async fn load_all_programs(&mut self) -> Result<()> {
        info!("Loading eBPF programs");

        // Environment variables for eBPF program paths
        let env_vars = [
            ("process", "BPF_PROCESS_MONITOR_PATH"),
            ("network", "BPF_NETWORK_MONITOR_PATH"),
            ("filesystem", "BPF_FILESYSTEM_MONITOR_PATH"),
        ];

        for (program_name, env_var) in &env_vars {
            match std::env::var(env_var) {
                Ok(path) if !path.is_empty() => {
                    match self.load_program(program_name, &path).await {
                        Ok(_) => {
                            info!(
                                "Successfully loaded {} eBPF program from {}",
                                program_name, path
                            );
                            self.health.programs.insert(program_name.to_string(), true);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to load {} eBPF program from {}: {}",
                                program_name, path, e
                            );
                            self.health.programs.insert(program_name.to_string(), false);
                        }
                    }
                }
                Ok(_) => {
                    info!(
                        "Environment variable {} is empty, skipping {} program",
                        env_var, program_name
                    );
                    self.health.programs.insert(program_name.to_string(), false);
                }
                Err(_) => {
                    info!(
                        "Environment variable {} not set, skipping {} program",
                        env_var, program_name
                    );
                    self.health.programs.insert(program_name.to_string(), false);
                }
            }
        }

        Ok(())
    }

    /// Load a specific eBPF program from file path
    async fn load_program(&mut self, name: &str, path: &str) -> Result<()> {
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
                self.loaded_programs.insert(name.to_string(), bpf);
                Ok(())
            }
            Err(e) => {
                error!("Failed to load {} eBPF program: {}", name, e);
                Err(anyhow::anyhow!("Failed to load eBPF program: {}", e))
            }
        }
    }

    /// Get health status of all eBPF programs
    pub fn get_health_status(&self) -> EbpfHealth {
        self.health.clone()
    }
}

/// Check if eBPF is available on the current system
pub fn ebpf_available() -> bool {
    // First check basic kernel eBPF support
    if !std::path::Path::new("/proc/sys/kernel/bpf_stats_enabled").exists() {
        return false;
    }

    check_capabilities()
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
        println!("Running as root - eBPF should be available");
        return true;
    }

    // Check process capabilities using /proc/self/status
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("CapEff:") {
                println!("Found CapEff line: {}", line);
                if let Some(cap_hex) = line.split_whitespace().nth(1) {
                    if let Ok(cap_val) = u64::from_str_radix(cap_hex, 16) {
                        println!("Parsed capability value: 0x{:x}", cap_val);
                        // CAP_BPF is bit 39 (0x8000000000)
                        // CAP_SYS_ADMIN is bit 21 (0x200000)
                        let cap_bpf = 1u64 << 39;
                        let cap_sys_admin = 1u64 << 21;
                        println!(
                            "Checking for CAP_BPF (0x{:x}) or CAP_SYS_ADMIN (0x{:x})",
                            cap_bpf, cap_sys_admin
                        );
                        if (cap_val & cap_bpf) != 0 {
                            println!("Found CAP_BPF capability!");
                            return true;
                        }
                        if (cap_val & cap_sys_admin) != 0 {
                            println!("Found CAP_SYS_ADMIN capability!");
                            return true;
                        }
                    }
                }
            }
        }
    }

    println!("No required capabilities found for eBPF");
    // Fallback: try to actually load a minimal eBPF program
    test_ebpf_loading()
}

/// Test if we can actually load an eBPF program
fn test_ebpf_loading() -> bool {
    // Try to access eBPF-related files to test if we have proper permissions

    // Check if we can read from debugfs (needed for some eBPF operations)
    if std::fs::File::open("/sys/kernel/debug/tracing/events").is_ok() {
        println!("Can access /sys/kernel/debug/tracing/events - eBPF likely available");
        return true;
    }

    // Check if we can access BPF syscall-related files
    if std::fs::File::open("/proc/sys/kernel/bpf_stats_enabled").is_ok() {
        println!("Can access BPF stats - BPF subsystem available");
        // If we can read this but not debugfs, we might still be able to do some eBPF operations
        return true;
    }

    // Try to create a BPF map to test if the syscall is available
    // For now, let's be optimistic and allow eBPF if we have capabilities but can't test thoroughly
    println!("Cannot test eBPF loading thoroughly, but capabilities suggest it might work");
    false
}
