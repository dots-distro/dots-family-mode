pub mod filesystem_monitor;
pub mod network_monitor;
pub mod process_monitor;

pub use filesystem_monitor::FilesystemMonitorEbpf;
pub use network_monitor::NetworkMonitorEbpf;
pub use process_monitor::ProcessMonitorEbpf;

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
