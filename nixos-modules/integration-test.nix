# NixOS Module Integration Test
# Tests the DOTS Family Mode NixOS module in a VM environment

{ pkgs, lib, config, ... }: {
  imports = [ 
    ./nixos-modules/dots-family/default.nix 
  ];

  # Enable DOTS Family Mode with test configuration
  services.dots-family = {
    enable = true;
    databasePath = "/tmp/test-family.db";
    reportingOnly = true;  # Safe for testing
    
    parentUsers = [ "testparent" ];
    childUsers = [ "testchild" ];
    
    enableWebFiltering = true;
    enableTerminalFiltering = false;
    enableNotifications = true;
    
    profiles.testchild = {
      name = "Test Child";
      ageGroup = "8-12";
      dailyScreenTimeLimit = "1h30m";
      
      timeWindows = [{
        start = "09:00";
        end = "17:00";
        days = [ "mon" "tue" "wed" "thu" "fri" ];
      }];
      
      allowedApplications = [ "firefox" "calculator" ];
      blockedApplications = [ "steam" ];
      webFilteringLevel = "strict";
    };
  };

  # Test user accounts
  users.users = {
    testparent = {
      isNormalUser = true;
      extraGroups = [ "wheel" "dots-family-parents" ];
      initialPassword = "parent123";
    };
    
    testchild = {
      isNormalUser = true;
      extraGroups = [ "dots-family-children" ];
      initialPassword = "child123";
    };
  };

  # Enable required services for testing
  services.xserver.enable = true;
  services.xserver.displayManager.lightdm.enable = true;
  services.dbus.enable = true;

  # Development packages for testing
  environment.systemPackages = with pkgs; [
    curl wget firefox
    strace gdb
    linuxPackages.bpftool
  ];

  # Virtual machine configuration
  virtualisation = {
    memorySize = 2048;
    cores = 2;
    diskSize = 8192;
    graphics = true;
    
    # Mount host directories for development
    sharedDirectories = {
      dots-source = {
        source = ".";
        target = "/mnt/dots-source";
      };
    };
  };

  # Test scripts
  environment.etc."dots-family/integration-test.sh" = {
    executable = true;
    text = ''
      #!/run/current-system/sw/bin/bash
      
      set -euo pipefail
      
      echo "DOTS Family Mode Integration Test"
      echo "================================="
      
      # Test 1: Service status
      echo "Testing systemd services..."
      systemctl is-active dots-family-daemon.service
      
      # Test 2: DBus interface
      echo "Testing DBus interface..."
      dbus-send --system --print-reply \
        --dest=org.dots.FamilyDaemon \
        /org/dots/FamilyDaemon \
        org.freedesktop.DBus.Introspectable.Introspect
      
      # Test 3: CLI tool
      echo "Testing CLI tool..."
      dots-family-ctl status
      dots-family-ctl profile list
      
      # Test 4: eBPF capabilities
      echo "Testing eBPF configuration..."
      ${pkgs.linuxPackages.bpftool}/bin/bpftool prog list || echo "No BPF programs loaded (expected)"
      
      # Test 5: Database
      echo "Testing database access..."
      ls -la /var/lib/dots-family/
      
      # Test 6: User groups
      echo "Testing user groups..."
      getent group dots-family-parents
      getent group dots-family-children
      groups testparent | grep dots-family-parents
      groups testchild | grep dots-family-children
      
      echo ""
      echo "Integration test completed successfully!"
    '';
  };

  # System configuration
  system.stateVersion = "25.05";
  
  # Boot configuration
  boot.loader.grub.enable = false;
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  
  # Network configuration
  networking.hostName = "dots-family-test";
  networking.useNetworkd = true;
  networking.useDHCP = false;
  systemd.network.enable = true;
  systemd.network.networks."10-lan" = {
    matchConfig.Type = "ether";
    networkConfig.DHCP = "yes";
  };
}