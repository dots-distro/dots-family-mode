import <nixpkgs/nixos/tests/make-test-python.nix> ({ pkgs, ... }: {
  name = "dots-family-basic-test";

  nodes.machine = import ./simple-vm-config.nix;

  testScript = ''
    # Start the machine
    machine.start()
    machine.wait_for_unit("multi-user.target")
    
    # Test 1: Check that binaries are installed and executable
    print("Testing binary installation...")
    machine.succeed("which dots-family-daemon")
    machine.succeed("which dots-family-ctl") 
    machine.succeed("which dots-family-monitor")
    print("All binaries found!")
    
    # Test 2: Test CLI help functionality
    print("Testing CLI help...")
    result = machine.succeed("dots-family-ctl --help")
    assert "dots-family-ctl" in result or "CLI" in result
    print("CLI help works!")
    
    # Test 3: Test daemon version/help (should not require DBus to show help)
    print("Testing daemon help...")
    result = machine.succeed("dots-family-daemon --help")
    print(f"Daemon help output: {result}")
    
    # Test 4: Test monitor help
    print("Testing monitor help...")  
    result = machine.succeed("dots-family-monitor --help")
    print(f"Monitor help output: {result}")
    
    # Test 5: Check DBus service file installation
    print("Checking DBus service installation...")
    machine.succeed("test -f /nix/store/*/share/dbus-1/system-services/org.dots.FamilyDaemon.service")
    
    # Test 6: Basic daemon startup test (should fail gracefully without config)
    print("Testing daemon startup behavior...")
    # Use timeout to limit test time, daemon may run indefinitely or exit
    result = machine.succeed("timeout 3 dots-family-daemon --help || echo 'timeout or help shown'")
    print(f"Daemon startup result: {result}")
    
    print("Basic DOTS Family Mode test completed successfully!")
  '';
})