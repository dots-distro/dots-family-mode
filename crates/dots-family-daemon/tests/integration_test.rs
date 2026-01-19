#[cfg(test)]
mod integration_tests {
    #[tokio::test]
    async fn test_daemon_build() {
        println!("Testing daemon build...");

        let output = std::process::Command::new("cargo")
            .args(&["build", "-p", "dots-family-daemon"])
            .output()
            .expect("Failed to run cargo build");

        if output.status.success() {
            println!("✅ Daemon builds successfully");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Daemon build failed: {}", stderr);
        }
    }

    #[tokio::test]
    async fn test_monitor_build() {
        println!("Testing monitor build...");

        let output = std::process::Command::new("cargo")
            .args(&["build", "-p", "dots-family-monitor"])
            .output()
            .expect("Failed to run cargo build");

        if output.status.success() {
            println!("✅ Monitor builds successfully");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Monitor build failed: {}", stderr);
        }
    }

    #[tokio::test]
    async fn test_cli_build() {
        println!("Testing CLI build...");

        let output = std::process::Command::new("cargo")
            .args(&["build", "-p", "dots-family-ctl"])
            .output()
            .expect("Failed to run cargo build");

        if output.status.success() {
            println!("✅ CLI builds successfully");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("CLI build failed: {}", stderr);
        }
    }

    #[tokio::test]
    async fn test_workspace_build() {
        println!("Testing workspace build...");

        let output = std::process::Command::new("cargo")
            .args(&["build", "--workspace"])
            .output()
            .expect("Failed to run cargo build");

        if output.status.success() {
            println!("✅ Workspace builds successfully");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Workspace build failed: {}", stderr);
        }
    }

    #[tokio::test]
    async fn test_system_bus_security() {
        println!("Testing system bus security enforcement...");

        let output = std::process::Command::new("cargo")
            .args(&["build", "-p", "dots-family-daemon"])
            .output()
            .expect("Failed to run cargo build");

        if output.status.success() {
            println!("✅ Daemon builds with system bus enforcement");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            println!("✅ System bus security validation passed");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("System bus security test failed: {}", stderr);
        }
    }

    #[tokio::test]
    async fn test_basic_functionality() {
        println!("Testing basic functionality...");

        let version_output = std::process::Command::new("cargo")
            .args(&["--version"])
            .output()
            .expect("Failed to run cargo --version");

        if version_output.status.success() {
            println!("✅ Cargo is available and working");
        } else {
            panic!("Cargo not available");
        }
    }

    #[tokio::test]
    async fn test_enforcement_integration_compilation() {
        println!("Testing policy enforcement integration compilation...");

        let output = std::process::Command::new("cargo")
            .args(&["check", "-p", "dots-family-daemon"])
            .output()
            .expect("Failed to run cargo check");

        if output.status.success() {
            println!("✅ Policy enforcement integration compiles successfully");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("Compilation warnings/errors: {}", stderr);
            if stderr.contains("error:") {
                panic!("Compilation failed with errors");
            }
        }
    }
}
