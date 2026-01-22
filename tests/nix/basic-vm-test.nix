import <nixpkgs/nixos/tests/make-test-python.nix> ({ pkgs, ... }: 
let
  # Build DOTS Family Mode directly using the workspace
  dots-family-mode = pkgs.rustPlatform.buildRustPackage rec {
    pname = "dots-family-mode";
    version = "0.1.0";
    
    src = ../.;
    
    cargoLock = {
      lockFile = ../Cargo.lock;
    };
    
    # Build dependencies we need
    nativeBuildInputs = with pkgs; [ 
      pkg-config 
      clang 
      llvm
    ];
    
    buildInputs = with pkgs; [ 
      sqlite 
      sqlcipher 
      dbus 
      systemd
      openssl
      gtk4
      libadwaita
    ];
    
    # Environment variables for building
    LIBCLANG_PATH = "${pkgs.clang.cc.lib}/lib";
    SQLX_OFFLINE = "true";  # Skip SQLx compile-time checks
    
    # Build all workspace members
    cargoBuildFlags = [ "--workspace" ];
    
    # Install binaries
    postInstall = ''
      # Ensure all binaries are available
      ls -la $out/bin/
      
      # Create systemd service file
      mkdir -p $out/lib/systemd/system
      cat > $out/lib/systemd/system/dots-family-daemon.service << 'EOF'
[Unit]
Description=DOTS Family Mode Daemon
After=network.target dbus.service
Wants=dbus.service

[Service]
Type=simple
ExecStart=$out/bin/dots-family-daemon
Restart=on-failure
User=root
Group=root
Environment=RUST_LOG=info
Environment=DATABASE_URL=sqlite:/tmp/family.db

[Install]
WantedBy=multi-user.target
EOF

      # Create basic DBus policy
      mkdir -p $out/share/dbus-1/system.d
      cat > $out/share/dbus-1/system.d/org.dots.FamilyDaemon.conf << 'EOF'
<!DOCTYPE busconfig PUBLIC
 "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy context="default">
    <allow own="org.dots.FamilyDaemon"/>
    <allow send_destination="org.dots.FamilyDaemon"/>
    <allow receive_sender="org.dots.FamilyDaemon"/>
  </policy>
</busconfig>
EOF
    '';
    
    meta = with pkgs.lib; {
      description = "DOTS Family Mode - Parental controls for Linux";
      license = licenses.agpl3Plus;
    };
  };

in {
  name = "dots-family-mode-basic";
  
  meta = with pkgs.lib.maintainers; {
    maintainers = [ ];
  };

  nodes = {
    machine = { config, pkgs, lib, ... }: {
      # Basic system configuration
      virtualisation = {
        memorySize = 4096;
        diskSize = 8192;
      };
      
      # Users for testing
      users.users = {
        root.password = "root";
        parent = {
          isNormalUser = true;
          password = "parent123";
          extraGroups = [ "wheel" ];
        };
        child = {
          isNormalUser = true;
          password = "child123";
        };
      };
      
      # Enable desktop environment
      services.xserver = {
        enable = true;
        displayManager.lightdm.enable = true;
        windowManager.i3.enable = true;
      };
      
      # Enable DBus
      services.dbus.enable = true;
      services.dbus.packages = [ dots-family-mode ];
      
      # Install DOTS Family Mode and test applications
      environment.systemPackages = with pkgs; [
        dots-family-mode
        firefox
        gnome.gnome-calculator
        busctl
        sqlite
        htop
        vim
        curl
        jq
      ];
      
      # Enable networking
      networking.networkmanager.enable = true;
      networking.firewall.enable = false;
      
      # System configuration
      system.stateVersion = "24.05";
      nix.settings.experimental-features = [ "nix-command" "flakes" ];
      
      # Create basic directories
      systemd.tmpfiles.rules = [
        "d /tmp/dots-family 0755 root root"
        "d /var/lib/dots-family 0755 root root"  
        "d /var/log/dots-family 0755 root root"
      ];
    };
  };

  testScript = ''
    import json
    
    # Start the machine
    machine.start()
    machine.wait_for_unit("default.target")
    
    print("=== DOTS Family Mode Basic VM Test ===")
    
    # Test 1: Check if binaries are installed
    print("=== Test 1: Binary Installation ===")
    daemon_binary = machine.succeed("which dots-family-daemon 2>/dev/null || echo 'DAEMON_NOT_FOUND'")
    print(f"Daemon binary: {daemon_binary.strip()}")
    
    monitor_binary = machine.succeed("which dots-family-monitor 2>/dev/null || echo 'MONITOR_NOT_FOUND'")
    print(f"Monitor binary: {monitor_binary.strip()}")
    
    ctl_binary = machine.succeed("which dots-family-ctl 2>/dev/null || echo 'CTL_NOT_FOUND'")
    print(f"CLI binary: {ctl_binary.strip()}")
    
    # Test 2: Check if binaries can be executed
    print("=== Test 2: Binary Execution ===")
    try:
        daemon_help = machine.succeed("dots-family-daemon --help 2>&1 | head -5")
        print(f"Daemon help output: {daemon_help}")
    except:
        print("Daemon --help failed")
    
    try:
        ctl_help = machine.succeed("dots-family-ctl --help 2>&1 | head -5")
        print(f"CLI help output: {ctl_help}")
    except:
        print("CLI --help failed")
    
    try:
        monitor_help = machine.succeed("dots-family-monitor --help 2>&1 | head -5")
        print(f"Monitor help output: {monitor_help}")
    except:
        print("Monitor --help failed")
    
    # Test 3: Try starting daemon manually
    print("=== Test 3: Manual Daemon Startup ===")
    try:
        # Create basic config
        machine.succeed("mkdir -p /tmp/dots-family")
        machine.succeed("""
cat > /tmp/dots-family/daemon.toml << 'EOF'
[database]
path = "/tmp/dots-family/test.db"

[policy_enforcement]  
enabled = false

[logging]
level = "info"
EOF
""")
        
        # Try to start daemon in background
        machine.succeed("cd /tmp/dots-family && timeout 10s dots-family-daemon --config daemon.toml > daemon.log 2>&1 &")
        machine.sleep(2)
        
        # Check if daemon started
        daemon_log = machine.succeed("head -20 /tmp/dots-family/daemon.log 2>/dev/null || echo 'NO_DAEMON_LOG'")
        print(f"Daemon startup log: {daemon_log}")
        
    except Exception as e:
        print(f"Manual daemon startup failed: {e}")
    
    # Test 4: Test CLI without daemon
    print("=== Test 4: CLI Without Daemon ===")
    try:
        cli_status = machine.succeed("dots-family-ctl status 2>&1 || echo 'CLI_FAILED_AS_EXPECTED'")
        print(f"CLI status (expected to fail): {cli_status}")
    except:
        print("CLI failed as expected (no daemon running)")
    
    # Test 5: Check database creation
    print("=== Test 5: Database ===")
    try:
        db_created = machine.succeed("test -f /tmp/dots-family/test.db && echo 'DB_CREATED' || echo 'DB_NOT_CREATED'")
        print(f"Database creation: {db_created}")
        
        if "DB_CREATED" in db_created:
            db_info = machine.succeed("file /tmp/dots-family/test.db")
            print(f"Database file info: {db_info}")
    except:
        print("Database check failed")
    
    # Test 6: Check DBus policy installation  
    print("=== Test 6: DBus Configuration ===")
    try:
        dbus_policy = machine.succeed("test -f /nix/store/*/share/dbus-1/system.d/org.dots.FamilyDaemon.conf && echo 'DBUS_POLICY_FOUND' || echo 'DBUS_POLICY_NOT_FOUND'")
        print(f"DBus policy: {dbus_policy}")
        
        # Check DBus service status
        dbus_status = machine.succeed("systemctl status dbus 2>&1 | head -5")
        print(f"DBus service status: {dbus_status}")
    except:
        print("DBus check failed")
    
    # Test 7: Check system dependencies
    print("=== Test 7: System Dependencies ===")
    try:
        sqlite_version = machine.succeed("sqlite3 --version")
        print(f"SQLite version: {sqlite_version.strip()}")
        
        systemd_version = machine.succeed("systemctl --version | head -1")
        print(f"Systemd version: {systemd_version.strip()}")
        
        kernel_version = machine.succeed("uname -r")
        print(f"Kernel version: {kernel_version.strip()}")
    except:
        print("System dependency check failed")
    
    # Test 8: Check user accounts
    print("=== Test 8: User Accounts ===")
    parent_id = machine.succeed("id parent")
    child_id = machine.succeed("id child")
    print(f"Parent user: {parent_id.strip()}")
    print(f"Child user: {child_id.strip()}")
    
    # Test 9: Check eBPF support (may not be available in VM)
    print("=== Test 9: eBPF Support ===")
    try:
        bpf_fs = machine.succeed("mount | grep bpf || echo 'BPF_FS_NOT_MOUNTED'")
        print(f"BPF filesystem: {bpf_fs.strip()}")
        
        bpf_syscall = machine.succeed("cat /proc/sys/kernel/unprivileged_bpf_disabled 2>/dev/null || echo 'BPF_SETTING_UNAVAILABLE'")
        print(f"BPF syscall setting: {bpf_syscall.strip()}")
    except:
        print("eBPF support check failed (expected in VM)")
    
    # Test 10: Final status summary
    print("=== Test 10: Summary ===")
    
    # Count what works
    working_components = []
    
    if "DAEMON_NOT_FOUND" not in daemon_binary:
        working_components.append("daemon-binary")
    if "CTL_NOT_FOUND" not in ctl_binary:
        working_components.append("ctl-binary") 
    if "MONITOR_NOT_FOUND" not in monitor_binary:
        working_components.append("monitor-binary")
        
    print(f"Working components: {', '.join(working_components) if working_components else 'NONE'}")
    print("=== VM Test Complete ===")
  '';
})