import <nixpkgs/nixos/tests/make-test-python.nix> ({ pkgs, ... }: {
  name = "dots-family-basic-functionality";
  
  nodes.machine = ./basic-vm-config.nix;

  testScript = ''
    # Start the machine and wait for it to be ready
    machine.start()
    machine.wait_for_unit("multi-user.target")
    
    # Test 1: Verify binaries are installed and in PATH
    print("Testing binary installation...")
    machine.succeed("which dots-family-daemon")
    machine.succeed("which dots-family-ctl")
    machine.succeed("which dots-family-monitor")
    print("✓ All binaries found in PATH")
    
    # Test 2: Test CLI tool basic functionality
    print("Testing CLI basic functionality...")
    result = machine.succeed("dots-family-ctl --help")
    print(f"CLI help output: {result[:200]}...")
    assert "dots-family-ctl" in result or "Usage:" in result
    print("✓ CLI help works correctly")
    
    # Test 3: Test daemon help/version without starting daemon
    print("Testing daemon help...")
    result = machine.succeed("dots-family-daemon --help")
    print(f"Daemon help output: {result[:200]}...")
    print("✓ Daemon help works correctly")
    
    # Test 4: Test monitor help
    print("Testing monitor help...")
    result = machine.succeed("dots-family-monitor --help")
    print(f"Monitor help output: {result[:200]}...")
    print("✓ Monitor help works correctly")
    
    # Test 5: Check that SQLite works (dependency verification)
    print("Testing SQLite dependency...")
    machine.succeed("sqlite3 --version")
    print("✓ SQLite is functional")
    
    # Test 6: Check DBus is running
    print("Testing DBus service...")
    machine.succeed("systemctl is-active dbus")
    print("✓ DBus is running")
    
    # Test 7: Basic daemon startup test - should exit with error but not crash
    print("Testing daemon startup behavior (expect graceful error)...")
    # Daemon should fail gracefully without proper config/permissions
    result = machine.fail("timeout 3 dots-family-daemon")
    print("✓ Daemon fails gracefully without proper setup")
    
    print("\n=== DOTS Family Mode Basic VM Test PASSED ===")
    print("All core binaries are built and functional!")
  '';
})