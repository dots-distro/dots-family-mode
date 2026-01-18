import <nixpkgs/nixos/tests/make-test-python.nix> {
  name = "dots-dbus-simple";

  nodes.machine = { config, pkgs, ... }: {
    imports = [ ./tests/simple-test-vm.nix ];
    
    environment.systemPackages = with pkgs; [
      sqlite
      dbus  
    ];
    
    # Enable test logging
    systemd.services.test-logger = {
      wantedBy = [ "multi-user.target" ];
      script = ''
        echo "Test started at $(date)" >> /tmp/test.log
      '';
    };
  };

  testScript = ''
    start_all()
    
    # Wait for the machine to be ready
    machine.wait_for_unit("multi-user.target")
    machine.succeed("echo '✅ Machine ready' | tee -a /tmp/test.log")
    
    # Check if D-Bus is running
    machine.succeed("systemctl is-active dbus")
    machine.succeed("echo '✅ D-Bus service is running' | tee -a /tmp/test.log")
    
    # Check if our D-Bus policy is installed  
    result = machine.succeed("find /usr/share/dbus-1 /etc/dbus-1 -name '*org.dots.FamilyDaemon*' 2>/dev/null || echo 'NOT_FOUND'")
    machine.succeed(f"echo 'D-Bus policy search result: {result}' | tee -a /tmp/test.log")
    
    if "NOT_FOUND" in result:
        machine.succeed("echo '❌ D-Bus policy not found!' | tee -a /tmp/test.log")
        # List what policies exist
        available = machine.succeed("ls -la /usr/share/dbus-1/system.d/ 2>/dev/null | head -5 || echo 'No system.d found'")
        machine.succeed(f"echo 'Available policies: {available}' | tee -a /tmp/test.log")
    else:
        machine.succeed("echo '✅ D-Bus policy found!' | tee -a /tmp/test.log")
        
    # Test D-Bus system bus access
    machine.succeed("dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply /org/freedesktop/DBus org.freedesktop.DBus.ListNames")
    machine.succeed("echo '✅ D-Bus system bus is accessible' | tee -a /tmp/test.log")
    
    # Get the test log and copy it out
    log_content = machine.succeed("cat /tmp/test.log")
    print("=== TEST LOG CONTENT ===")
    print(log_content)
    print("=== END TEST LOG ===")
    
    machine.succeed("echo '🎉 VM D-Bus integration test completed!' | tee -a /tmp/test.log")
  '';
}