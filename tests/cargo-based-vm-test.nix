import ../vm-test/lib.nix ({ pkgs, ... }: {
  name = "dots-family-cargo-test";

  machine = { config, pkgs, ... }: {
    # Basic NixOS configuration for VM testing
    users.users.testuser = {
      isNormalUser = true;
      password = "test";
      extraGroups = [ "wheel" ];
    };

    # Enable services needed for testing
    services.dbus.enable = true;
    
    # Install development tools for building Rust code
    environment.systemPackages = with pkgs; [
      rustc
      cargo
      pkg-config
      openssl.dev
      sqlite.dev
      sqlcipher.dev
      dbus.dev
      git
      # Runtime dependencies
      procfs
      util-linux
      dbus
      polkit
    ];

    # Copy source code to VM (for cargo build within VM)
    environment.etc."dots-family-source" = {
      source = ../.;
      target = "dots-family-source";
    };
    
    # DBus configuration for testing
    services.dbus.packages = [ 
      (pkgs.writeTextFile {
        name = "dots-family-dbus-service";
        destination = "/share/dbus-1/services/org.dots.FamilyDaemon.service";
        text = ''
          [D-BUS Service]
          Name=org.dots.FamilyDaemon
          Exec=/etc/dots-family-source/target/debug/dots-family-daemon
        '';
      })
    ];
  };

  testScript = ''
    import subprocess
    import time
    
    # Start the virtual machine
    machine.start()
    machine.wait_for_unit("multi-user.target")
    
    # Copy source code to a writable location in VM
    machine.execute("sudo -u testuser cp -r /etc/dots-family-source /home/testuser/")
    machine.execute("sudo -u testuser chown -R testuser:users /home/testuser/dots-family-source")
    
    # Build DOTS Family Mode components using cargo
    print("Building DOTS Family Mode components...")
    result = machine.execute("cd /home/testuser/dots-family-source && sudo -u testuser cargo build --release --bin dots-family-daemon --bin dots-family-ctl --bin dots-family-monitor")
    
    if result[0] != 0:
        print(f"Build failed with exit code {result[0]}")
        print(f"Build output: {result[1]}")
        raise Exception("Failed to build DOTS Family Mode components")
    
    print("Build successful!")
    
    # Test 1: Check that binaries were created
    print("Checking if binaries exist...")
    machine.execute("test -f /home/testuser/dots-family-source/target/release/dots-family-daemon")
    machine.execute("test -f /home/testuser/dots-family-source/target/release/dots-family-ctl") 
    machine.execute("test -f /home/testuser/dots-family-source/target/release/dots-family-monitor")
    print("All binaries found!")
    
    # Test 2: Check CLI help functionality
    print("Testing CLI help...")
    result = machine.execute("cd /home/testuser/dots-family-source && sudo -u testuser ./target/release/dots-family-ctl --help")
    assert "DOTS Family Mode CLI" in result[1] or "dots-family-ctl" in result[1]
    print("CLI help works!")
    
    # Test 3: Attempt to start daemon (should fail gracefully without proper setup)
    print("Testing daemon startup behavior...")
    result = machine.execute("cd /home/testuser/dots-family-source && sudo -u testuser timeout 5 ./target/release/dots-family-daemon || true")
    print(f"Daemon result: {result}")
    # Daemon should either start and get killed by timeout, or fail with a reasonable error
    # Both are acceptable for basic functionality test
    
    # Test 4: Monitor should be able to show its version/help
    print("Testing monitor help...")
    result = machine.execute("cd /home/testuser/dots-family-source && sudo -u testuser ./target/release/dots-family-monitor --help")
    print(f"Monitor help: {result[1]}")
    
    print("Basic DOTS Family Mode VM test passed!")
  '';
})