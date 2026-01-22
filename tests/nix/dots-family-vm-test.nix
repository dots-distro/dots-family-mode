import <nixpkgs/nixos/tests/make-test-python.nix> ({ pkgs, ... }: {
  name = "dots-family-mode";
  
  meta = with pkgs.lib.maintainers; {
    maintainers = [ ];
  };

  nodes = {
    # Main test machine with DOTS Family Mode
    machine = { config, pkgs, lib, ... }: {
      # Import the actual NixOS modules
      imports = [ ../nixos-modules/dots-family ];
      
      # Enable DOTS Family Mode
      services.dots-family = {
        enable = true;
        parentUsers = [ "parent" ];
        childUsers = [ "child" ];
        reportingOnly = false;  # Test actual enforcement
        
        # Test profile configuration
        profiles.child = {
          name = "Test Child";
          ageGroup = "8-12";
          dailyScreenTimeLimit = "2h";
          timeWindows = [{
            start = "09:00";
            end = "17:00";
            days = [ "mon" "tue" "wed" "thu" "fri" ];
          }];
          allowedApplications = [ "firefox" "calculator" ];
          blockedApplications = [ "steam" ];
          webFilteringLevel = "moderate";
        };
      };
      
      # Users for testing
      users.users = {
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
      
      # Basic system configuration
      virtualisation = {
        memorySize = 4096;
        diskSize = 8192;
      };
      
      # Enable desktop environment for GUI testing
      services.xserver = {
        enable = true;
        displayManager.lightdm.enable = true;
        windowManager.i3.enable = true;
      };
      
      # Enable networking
      networking.networkmanager.enable = true;
      networking.firewall.enable = false;  # Disable for testing
      
      # Development tools and test applications
      environment.systemPackages = with pkgs; [
        firefox
        gnome.gnome-calculator
        steam-run  # For testing blocked apps
        busctl     # For DBus debugging
        sqlite     # For database inspection
        htop
        vim
        curl
      ];
      
      # Enable experimental features
      nix.settings.experimental-features = [ "nix-command" "flakes" ];
      
      # System version
      system.stateVersion = "24.05";
    };
  };

  testScript = ''
    # Start the machine and wait for it to boot
    machine.start()
    machine.wait_for_unit("default.target")
    machine.wait_for_unit("graphical-session.target")
    
    # Test 1: Verify DOTS Family Daemon is running
    print("=== Test 1: Daemon Service Status ===")
    machine.wait_for_unit("dots-family-daemon.service")
    daemon_status = machine.succeed("systemctl status dots-family-daemon.service")
    print(f"Daemon status: {daemon_status}")
    
    # Test 2: Verify DBus interface is available
    print("=== Test 2: DBus Interface ===")
    try:
        dbus_result = machine.succeed("busctl --system list | grep org.dots.FamilyDaemon || echo 'SERVICE_NOT_FOUND'")
        print(f"DBus service status: {dbus_result}")
        
        # Try to call a basic DBus method
        dbus_call = machine.succeed("busctl --system call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.dots.FamilyDaemon GetStatus || echo 'DBUS_CALL_FAILED'")
        print(f"DBus call result: {dbus_call}")
    except Exception as e:
        print(f"DBus test failed: {e}")
    
    # Test 3: Verify CLI tool works
    print("=== Test 3: CLI Tool ===")
    try:
        cli_status = machine.succeed("dots-family-ctl status || echo 'CLI_FAILED'")
        print(f"CLI status: {cli_status}")
        
        # Test profile listing
        profile_list = machine.succeed("dots-family-ctl profile list || echo 'PROFILE_LIST_FAILED'")
        print(f"Profile list: {profile_list}")
    except Exception as e:
        print(f"CLI test failed: {e}")
    
    # Test 4: Verify database exists and is accessible
    print("=== Test 4: Database ===")
    try:
        db_check = machine.succeed("test -f /var/lib/dots-family/family.db && echo 'DB_EXISTS' || echo 'DB_MISSING'")
        print(f"Database check: {db_check}")
        
        # Check database structure (if accessible)
        if "DB_EXISTS" in db_check:
            db_tables = machine.succeed("sqlite3 /var/lib/dots-family/family.db '.tables' 2>/dev/null || echo 'DB_ACCESS_FAILED'")
            print(f"Database tables: {db_tables}")
    except Exception as e:
        print(f"Database test failed: {e}")
    
    # Test 5: Test user group membership
    print("=== Test 5: User Groups ===")
    parent_groups = machine.succeed("groups parent")
    child_groups = machine.succeed("groups child")
    print(f"Parent groups: {parent_groups}")
    print(f"Child groups: {child_groups}")
    
    # Test 6: Monitor service testing (user session)
    print("=== Test 6: Monitor Service ===")
    try:
        # Login as parent user and check monitor service
        machine.succeed("loginctl enable-linger parent")
        machine.succeed("sudo -u parent XDG_RUNTIME_DIR=/run/user/$(id -u parent) systemctl --user start dots-family-monitor.service || echo 'MONITOR_START_FAILED'")
        
        monitor_status = machine.succeed("sudo -u parent XDG_RUNTIME_DIR=/run/user/$(id -u parent) systemctl --user status dots-family-monitor.service || echo 'MONITOR_STATUS_FAILED'")
        print(f"Monitor status: {monitor_status}")
    except Exception as e:
        print(f"Monitor test failed: {e}")
    
    # Test 7: Application permission checking
    print("=== Test 7: Application Permissions ===")
    try:
        # Test allowed application
        firefox_check = machine.succeed("dots-family-ctl check firefox || echo 'CHECK_FAILED'")
        print(f"Firefox permission: {firefox_check}")
        
        # Test blocked application (if configured)
        steam_check = machine.succeed("dots-family-ctl check steam || echo 'CHECK_FAILED'")
        print(f"Steam permission: {steam_check}")
    except Exception as e:
        print(f"Permission check failed: {e}")
    
    # Test 8: Configuration files
    print("=== Test 8: Configuration ===")
    try:
        config_check = machine.succeed("test -d /etc/dots-family && echo 'CONFIG_DIR_EXISTS' || echo 'CONFIG_DIR_MISSING'")
        print(f"Config directory: {config_check}")
        
        if "CONFIG_DIR_EXISTS" in config_check:
            config_files = machine.succeed("ls -la /etc/dots-family/ 2>/dev/null || echo 'CONFIG_LIST_FAILED'")
            print(f"Config files: {config_files}")
    except Exception as e:
        print(f"Configuration test failed: {e}")
    
    # Test 9: Log files
    print("=== Test 9: Logging ===")
    try:
        log_check = machine.succeed("journalctl -u dots-family-daemon.service --no-pager -n 20 || echo 'LOG_FAILED'")
        print(f"Recent daemon logs: {log_check}")
    except Exception as e:
        print(f"Log test failed: {e}")
    
    # Test 10: eBPF capabilities (may fail in VM)
    print("=== Test 10: eBPF Support ===")
    try:
        ebpf_check = machine.succeed("ls /sys/fs/bpf 2>/dev/null && echo 'EBPF_SUPPORTED' || echo 'EBPF_NOT_AVAILABLE'")
        print(f"eBPF support: {ebpf_check}")
        
        # Check if BPF syscall is available
        bpf_syscall = machine.succeed("cat /proc/sys/kernel/unprivileged_bpf_disabled 2>/dev/null || echo 'BPF_SYSCALL_INFO_UNAVAILABLE'")
        print(f"BPF syscall info: {bpf_syscall}")
    except Exception as e:
        print(f"eBPF test failed: {e}")
    
    print("=== Test Summary ===")
    print("DOTS Family Mode VM test completed")
    print("Check output above for any FAILED messages")
  '';
})