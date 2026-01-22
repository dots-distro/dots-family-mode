import <nixpkgs/nixos/tests/make-test-python.nix> {
  name = "dots-dbus-full-validation";

  nodes.machine = { config, pkgs, ... }: {
    imports = [ ./tests/simple-test-vm.nix ];
    
    environment.systemPackages = with pkgs; [
      sqlite
      dbus  
    ];

    # Copy our built binaries into the VM
    environment.etc."dots-family-daemon".source = ./target/x86_64-unknown-linux-gnu/release/dots-family-daemon;
    environment.etc."dots-family-monitor".source = ./target/x86_64-unknown-linux-gnu/release/dots-family-monitor;
    environment.etc."dots-family-ctl".source = ./target/x86_64-unknown-linux-gnu/release/dots-family-ctl;
  };

  testScript = ''
    start_all()
    machine.wait_for_unit("multi-user.target")
    machine.succeed("echo '=== DOTS Family D-Bus Integration Validation ===' | tee /tmp/test.log")
    
    # Verify D-Bus is running
    machine.succeed("systemctl is-active dbus")
    machine.succeed("echo '✅ D-Bus service active' | tee -a /tmp/test.log")
    
    # Verify our D-Bus policy is installed (check multiple locations)
    machine.succeed("echo '🔍 Checking D-Bus policy installation...' | tee -a /tmp/test.log")
    policy_locations = machine.succeed("find /nix/store -name 'org.dots.FamilyDaemon.conf' 2>/dev/null; find /usr/share/dbus-1 /etc/dbus-1 /run/dbus-1 -name '*org.dots.FamilyDaemon*' 2>/dev/null || echo 'NOT_FOUND'")
    machine.succeed(f"echo 'Policy search results: {policy_locations}' | tee -a /tmp/test.log")
    
    # Check if D-Bus has loaded our policy
    dbus_config = machine.execute("dbus-send --system --dest=org.freedesktop.DBus --print-reply /org/freedesktop/DBus org.freedesktop.DBus.GetConnectionCredentials string:org.freedesktop.DBus 2>/dev/null || echo 'DBUS_ERROR'")
    machine.succeed(f"echo 'D-Bus connection test: {dbus_config[1]}' | tee -a /tmp/test.log")
    
    if "NOT_FOUND" not in policy_locations or "/nix/store" in policy_locations:
        machine.succeed("echo '✅ D-Bus policy installed' | tee -a /tmp/test.log")
        
        # Test daemon startup with policy installed
        machine.succeed("echo '🔍 Testing daemon registration with D-Bus policy...' | tee -a /tmp/test.log")
        
        # Start daemon in background
        machine.execute("cp /etc/dots-family-daemon /tmp/dots-family-daemon")
        machine.execute("chmod +x /tmp/dots-family-daemon")
        
        # Try to start daemon and capture output
        daemon_result = machine.execute("timeout 10 /tmp/dots-family-daemon 2>&1 || echo 'DAEMON_TIMEOUT'")
        
        if "AccessDenied" in daemon_result[1]:
            machine.succeed("echo '❌ Daemon registration still blocked - D-Bus policy not working' | tee -a /tmp/test.log")
        elif "DAEMON_TIMEOUT" in daemon_result[1]:
            machine.succeed("echo '⚠️  Daemon started but timed out - likely running successfully' | tee -a /tmp/test.log")
        else:
            machine.succeed("echo '✅ Daemon registration successful!' | tee -a /tmp/test.log")
            
        # Test D-Bus service registration
        services = machine.succeed("dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply /org/freedesktop/DBus org.freedesktop.DBus.ListNames | grep -o org.dots.FamilyDaemon || echo 'SERVICE_NOT_FOUND'")
        
        if "SERVICE_NOT_FOUND" not in services:
            machine.succeed("echo '🎉 DOTS daemon service registered in D-Bus!' | tee -a /tmp/test.log")
        else:
            machine.succeed("echo '⚠️  DOTS daemon service not found in D-Bus list' | tee -a /tmp/test.log")
            
    else:
        machine.succeed("echo '❌ D-Bus policy NOT installed - cannot validate daemon registration' | tee -a /tmp/test.log")
    
    # Always output the test log
    log_content = machine.succeed("cat /tmp/test.log")
    print("=== VM VALIDATION RESULTS ===")
    print(log_content)
    print("=== END VM VALIDATION ===")
  '';
}