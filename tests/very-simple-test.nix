import <nixpkgs/nixos/tests/make-test-python.nix> ({ pkgs, ... }: {
  name = "dots-family-simple";
  
  nodes = {
    machine = { config, pkgs, lib, ... }: {
      virtualisation.memorySize = 1024;
      
      users.users.root = {
        hashedPassword = lib.mkForce null;
        password = lib.mkForce "root";
      };
      
      environment.systemPackages = with pkgs; [ 
        sqlite 
        systemd
      ];
      
      services.dbus.enable = true;
      system.stateVersion = "24.05";
    };
  };

  testScript = ''
    machine.start()
    machine.wait_for_unit("default.target")
    
    print("Basic VM test started")
    
    # Test 1: Check basic system
    version = machine.succeed("uname -r")
    print(f"Kernel: {version.strip()}")
    
    sqlite_version = machine.succeed("sqlite3 --version")
    print(f"SQLite: {sqlite_version.strip()}")
    
    dbus_status = machine.succeed("systemctl is-active dbus")
    print(f"DBus: {dbus_status.strip()}")
    
    print("Basic VM test completed successfully")
  '';
})