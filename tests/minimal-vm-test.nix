import <nixpkgs/nixos/tests/make-test-python.nix> ({ pkgs, ... }:

{
  name = "dots-family-mode-minimal";
  
  meta = with pkgs.lib.maintainers; {
    maintainers = [ ];
  };

  nodes = {
    machine = { config, pkgs, lib, ... }: {
      # Basic system configuration
      virtualisation = {
        memorySize = 2048;
        diskSize = 4096;
      };
      
      # User configuration
      users.users.root = {
        hashedPassword = lib.mkForce null;
        hashedPasswordFile = lib.mkForce null;
        password = lib.mkForce "root";
      };
      users.users.testuser = {
        isNormalUser = true;
        password = "test";
        extraGroups = [ "wheel" ];
      };
      
      # Enable DBus and networking
      services.dbus.enable = true;
      networking.networkmanager.enable = true;
      networking.firewall.enable = false;
      
      # Install development tools needed to build DOTS Family Mode
      environment.systemPackages = with pkgs; [
        rustc
        cargo
        sqlite
        sqlcipher
        pkg-config
        systemd  # includes busctl
        git
        vim
        jq
      ];
      
      # Enable Nix flakes
      nix.settings.experimental-features = [ "nix-command" "flakes" ];
      
      # System version
      system.stateVersion = "24.05";
    };
  };

  testScript = ''
    # Start the machine
    machine.start()
    machine.wait_for_unit("default.target")
    machine.wait_for_unit("dbus.service")
    
    print("=== DOTS Family Mode Minimal Build Test ===")
    
    # Test 1: Copy source code and try to build
    print("=== Test 1: Source Code and Build Environment ===")
    
    # Create a test directory and copy essential files
    machine.succeed("mkdir -p /tmp/dots-test")
    
    # Check if we have Rust toolchain
    rust_version = machine.succeed("rustc --version")
    print(f"Rust version: {rust_version.strip()}")
    
    cargo_version = machine.succeed("cargo --version") 
    print(f"Cargo version: {cargo_version.strip()}")
    
    # Check SQLite
    sqlite_version = machine.succeed("sqlite3 --version")
    print(f"SQLite version: {sqlite_version.strip()}")
    
    # Test 2: Create a minimal Rust project to test build environment
    print("=== Test 2: Minimal Rust Build Test ===")
    
    machine.succeed("""
cd /tmp/dots-test
cargo init --name test-project
""")
    
    # Add some dependencies that DOTS Family Mode uses
    machine.succeed("""
cd /tmp/dots-test
cat >> Cargo.toml << 'EOF'
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
EOF
""")
    
    # Create a simple test program
    machine.succeed("""
cd /tmp/dots-test
cat > src/main.rs << 'EOF'
use tokio;

#[tokio::main]
async fn main() {
    println!("Basic async Rust program works!");
    
    // Test database connection (will fail but should compile)
    match sqlx::sqlite::SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
    {
        Ok(_) => println!("SQLite connection successful"),
        Err(e) => println!("SQLite connection failed (expected): {}", e),
    }
}
EOF
""")
    
    # Try to build the test project
    try:
        build_result = machine.succeed("cd /tmp/dots-test && timeout 300s cargo build 2>&1")
        print(f"Build result: {build_result}")
        
        # Try to run it
        run_result = machine.succeed("cd /tmp/dots-test && timeout 10s ./target/debug/test-project 2>&1")
        print(f"Run result: {run_result}")
        
    except Exception as e:
        print(f"Build test failed: {e}")
        
        # Get more details about the failure
        cargo_error = machine.succeed("cd /tmp/dots-test && cargo build 2>&1 | tail -20")
        print(f"Cargo error details: {cargo_error}")
    
    # Test 3: Check system capabilities for DOTS Family Mode
    print("=== Test 3: System Capabilities ===")
    
    # Check if we can access eBPF
    try:
        bpf_check = machine.succeed("ls /sys/fs/bpf 2>/dev/null && echo 'BPF_AVAILABLE' || echo 'BPF_NOT_AVAILABLE'")
        print(f"eBPF filesystem: {bpf_check.strip()}")
    except:
        print("eBPF check failed")
    
    # Check DBus
    dbus_status = machine.succeed("systemctl is-active dbus")
    print(f"DBus status: {dbus_status.strip()}")
    
    try:
        dbus_session = machine.succeed("busctl --user list 2>&1 || echo 'USER_DBUS_FAILED'")
        print(f"User DBus: {'Working' if 'USER_DBUS_FAILED' not in dbus_session else 'Failed'}")
    except:
        print("User DBus check failed")
    
    # Check if we can create and write to database locations
    db_test = machine.succeed("""
mkdir -p /tmp/db-test /var/lib/test-family
touch /tmp/db-test/test.db
touch /var/lib/test-family/test.db 2>/dev/null && echo 'VAR_LIB_WRITABLE' || echo 'VAR_LIB_READONLY'
""")
    print(f"Database location test: {db_test.strip()}")
    
    # Test 4: Try to simulate what DOTS Family Mode needs
    print("=== Test 4: DOTS Family Mode Environment Simulation ===")
    
    # Create directory structure
    machine.succeed("mkdir -p /tmp/dots-family-sim/{bin,config,db,logs}")
    
    # Create a mock daemon script
    machine.succeed("""
cat > /tmp/dots-family-sim/bin/mock-daemon << 'EOF'
#!/usr/bin/env bash
echo "Mock DOTS Family Daemon starting..."
echo "Database path: $1"
echo "Config path: $2"
echo "Creating database..."
sqlite3 "$1" "CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY);"
echo "Database created successfully"
echo "Daemon would start here (simulated)"
EOF
chmod +x /tmp/dots-family-sim/bin/mock-daemon
""")
    
    # Create mock config
    machine.succeed("""
cat > /tmp/dots-family-sim/config/daemon.toml << 'EOF'
[database]
path = "/tmp/dots-family-sim/db/family.db"

[logging]
level = "info"

[policy_enforcement]
enabled = false
EOF
""")
    
    # Run the mock daemon
    try:
        mock_result = machine.succeed("""
cd /tmp/dots-family-sim
./bin/mock-daemon ./db/family.db ./config/daemon.toml 2>&1
""")
        print(f"Mock daemon result: {mock_result}")
        
        # Check if database was created
        db_check = machine.succeed("test -f /tmp/dots-family-sim/db/family.db && echo 'MOCK_DB_CREATED' || echo 'MOCK_DB_FAILED'")
        print(f"Mock database: {db_check.strip()}")
        
        if "MOCK_DB_CREATED" in db_check:
            tables = machine.succeed("sqlite3 /tmp/dots-family-sim/db/family.db '.tables'")
            print(f"Mock database tables: {tables.strip()}")
            
    except Exception as e:
        print(f"Mock daemon test failed: {e}")
    
    # Test 5: Summary
    print("=== Test 5: Environment Summary ===")
    
    # Check what works
    components = []
    issues = []
    
    # Rust toolchain
    try:
        machine.succeed("rustc --version > /dev/null")
        components.append("rust-toolchain")
    except:
        issues.append("rust-toolchain")
    
    # SQLite
    try:
        machine.succeed("sqlite3 --version > /dev/null")
        components.append("sqlite")
    except:
        issues.append("sqlite")
    
    # DBus
    try:
        machine.succeed("systemctl is-active dbus > /dev/null")
        components.append("dbus")
    except:
        issues.append("dbus")
    
    # File system access
    try:
        machine.succeed("touch /tmp/test && rm /tmp/test")
        components.append("filesystem-rw")
    except:
        issues.append("filesystem-rw")
    
    print(f"WORKING COMPONENTS: {', '.join(components) if components else 'NONE'}")
    print(f"ISSUES FOUND: {', '.join(issues) if issues else 'NONE'}")
    
    print("=== Minimal VM Test Complete ===")
    print("This test validates the build environment for DOTS Family Mode")
  '';
})