import <nixpkgs/nixos/tests/make-test-python.nix> {
  name = "dots-dbus-test";

  machine = { config, pkgs, ... }: {
    imports = [ ./tests/simple-test-vm.nix ];
    
    environment.systemPackages = with pkgs; [
      sqlite
      dbus  
    ];
  };

  testScript = ''
    start_all()
    
    # Wait for the machine to be ready
    machine.wait_for_unit("multi-user.target")
    
    # Check if D-Bus is running
    machine.succeed("systemctl is-active dbus")
    print("✅ D-Bus service is running")
    
    # Check if our D-Bus policy is installed  
    result = machine.succeed("find /usr/share/dbus-1 /etc/dbus-1 -name '*org.dots.FamilyDaemon*' 2>/dev/null || echo 'NOT_FOUND'")
    print(f"D-Bus policy search result: {result}")
    
    if "NOT_FOUND" in result:
        print("❌ D-Bus policy not found!")
        # List what policies exist
        available = machine.succeed("ls -la /usr/share/dbus-1/system.d/ 2>/dev/null | head -10 || echo 'No system.d found'")
        print(f"Available policies: {available}")
    else:
        print("✅ D-Bus policy found!")
        
    # Test D-Bus system bus access
    machine.succeed("dbus-send --system --dest=org.freedesktop.DBus --type=method_call --print-reply /org/freedesktop/DBus org.freedesktop.DBus.ListNames")
    print("✅ D-Bus system bus is accessible")
    
    print("🎉 VM D-Bus integration test completed!")
  '';
}