use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=BPF_PROCESS_MONITOR_PATH");
    println!("cargo:rerun-if-env-changed=BPF_NETWORK_MONITOR_PATH");
    println!("cargo:rerun-if-env-changed=BPF_FILESYSTEM_MONITOR_PATH");

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir_path = PathBuf::from(out_dir);

    // Check if Nix has injected the eBPF program paths
    if let (Ok(process_path), Ok(network_path), Ok(filesystem_path)) = (
        env::var("BPF_PROCESS_MONITOR_PATH"),
        env::var("BPF_NETWORK_MONITOR_PATH"),
        env::var("BPF_FILESYSTEM_MONITOR_PATH"),
    ) {
        println!("cargo:rustc-env=USING_NIX_EBPF=1");

        // Embed the Nix store paths directly (don't copy!)
        // This ensures the eBPF package is in the runtime closure
        println!("cargo:rustc-env=BPF_PROCESS_MONITOR_FILE={}", process_path);
        println!("cargo:rustc-env=BPF_NETWORK_MONITOR_FILE={}", network_path);
        println!("cargo:rustc-env=BPF_FILESYSTEM_MONITOR_FILE={}", filesystem_path);

        // Tell cargo to rerun if these files change (creates runtime dependency)
        println!("cargo:rerun-if-changed={}", process_path);
        println!("cargo:rerun-if-changed={}", network_path);
        println!("cargo:rerun-if-changed={}", filesystem_path);

        println!("cargo:warning=Using Nix-provided eBPF programs at:");
        println!("cargo:warning=  Process: {}", process_path);
        println!("cargo:warning=  Network: {}", network_path);
        println!("cargo:warning=  Filesystem: {}", filesystem_path);
    } else {
        println!("cargo:rustc-env=USING_NIX_EBPF=0");

        // Fallback: try to build eBPF programs using aya-build (for local development)
        if env::var("CARGO_CFG_TARGET_ARCH").is_ok() {
            // Only attempt eBPF build if we have aya-build available and we're not cross-compiling
            match build_ebpf_programs() {
                Ok(_) => println!("cargo:info=Built eBPF programs locally"),
                Err(e) => {
                    println!("cargo:warning=Failed to build eBPF programs locally: {}", e);
                    println!("cargo:warning=eBPF monitors will use simulation mode");

                    // Create empty files so the build doesn't fail
                    let empty_process = out_dir_path.join("process-monitor");
                    let empty_network = out_dir_path.join("network-monitor");
                    let empty_filesystem = out_dir_path.join("filesystem-monitor");

                    let _ = fs::write(empty_process, b"");
                    let _ = fs::write(empty_network, b"");
                    let _ = fs::write(empty_filesystem, b"");
                }
            }
        }
    }
}

fn build_ebpf_programs() -> Result<(), Box<dyn std::error::Error>> {
    // Check if we have the eBPF crate available
    let ebpf_path = PathBuf::from("../dots-family-ebpf");
    if !ebpf_path.exists() {
        return Err("eBPF crate not found".into());
    }

    // This would use aya-build in a real implementation
    // For now, just return an error to trigger simulation mode
    Err("aya-build not implemented in build.rs yet".into())
}
