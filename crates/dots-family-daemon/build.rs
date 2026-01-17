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

        // Copy Nix-built eBPF programs to expected locations
        let process_dest = out_dir_path.join("process-monitor");
        let network_dest = out_dir_path.join("network-monitor");
        let filesystem_dest = out_dir_path.join("filesystem-monitor");

        if fs::metadata(&process_path).is_ok() {
            fs::copy(&process_path, &process_dest)
                .unwrap_or_else(|e| panic!("Failed to copy process monitor eBPF: {}", e));
            println!("cargo:rustc-env=BPF_PROCESS_MONITOR_FILE={}", process_dest.display());
        }

        if fs::metadata(&network_path).is_ok() {
            fs::copy(&network_path, &network_dest)
                .unwrap_or_else(|e| panic!("Failed to copy network monitor eBPF: {}", e));
            println!("cargo:rustc-env=BPF_NETWORK_MONITOR_FILE={}", network_dest.display());
        }

        if fs::metadata(&filesystem_path).is_ok() {
            fs::copy(&filesystem_path, &filesystem_dest)
                .unwrap_or_else(|e| panic!("Failed to copy filesystem monitor eBPF: {}", e));
            println!("cargo:rustc-env=BPF_FILESYSTEM_MONITOR_FILE={}", filesystem_dest.display());
        }

        println!("cargo:info=Using Nix-provided eBPF programs");
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
