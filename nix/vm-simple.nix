# Simplified NixOS VM configuration for testing DOTS Family Mode
{ config, pkgs, lib, modulesPath, ... }:

{
  imports = [ "${modulesPath}/virtualisation/qemu-vm.nix" ];

  # Basic system configuration
  time.timeZone = "UTC";
  i18n.defaultLocale = "en_US.UTF-8";
  
  # Enable experimental features for development
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # Users
  users.users = {
    root.password = "root";
    
    # Parent user (admin)
    parent = {
      isNormalUser = true;
      password = "parent123";
      extraGroups = [ "wheel" "networkmanager" "audio" "video" ];
      shell = pkgs.bash;
    };
    
    # Child user (monitored)
    child = {
      isNormalUser = true;
      password = "child123";
      extraGroups = [ "audio" "video" ];
      shell = pkgs.bash;
    };
  };

  # Enable SSH for remote access with proper networking
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PermitUserEnvironment = "yes";
      PasswordAuthentication = true;
      ChallengeResponseAuthentication = false;
      KbdInteractiveAuthentication = false;
      IgnoreUserKnownHosts = true;
      IgnoreRhosts = true;
      StrictModes = false;
      ListenAddress = "0.0.0.0";
      Port = 22;
    };
  };

  # Open firewall port for SSH (though VM firewall is disabled)
  networking.firewall.allowedTCPPorts = [ 22 ];

  # Enable QEMU guest agent for better VM communication
  services.qemuGuest.enable = true;

  # Enable greetd display manager with autologin to terminal
  services.greetd = {
    enable = true;
    settings = {
      default_session = {
        command = "${pkgs.dbus}/bin/dbus-run-session ${pkgs.bash}/bin/bash -c 'export HOME=/home/parent && exec ${pkgs.foot}/bin/foot'";
        user = "parent";
      };
    };
  };

  # Run automated tests on boot
  systemd.services.dots-family-vm-test = {
    description = "DOTS Family Mode - Automated VM Test";
    after = [ "network.target" "dbus.service" "graphical-session.target" ];
    wants = [ "network.target" "dbus.service" ];
    wantedBy = [ "multi-user.target" ];
    
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = "yes";
      ExecStart = "${pkgs.bash}/bin/bash -c 'sleep 15 && /tmp/run-web-filtering-test.sh --evidence /var/log/dots-family-web-test'";
      User = "parent";
      Group = "parent";
    };
  };

  # Create evidence directory
  environment.etc."dots-family/evidence".text = ''
    DOTS Family Mode Web Filtering Test Evidence
    Location: /var/log/dots-family-web-test
  '';

  # Install required packages
  environment.systemPackages = with pkgs; [
    firefox
    foot
    alacritty
    sqlite
    systemd  # Provides busctl and systemctl
    util-linux  # Provides journalctl
    htop
    vim
    git
    curl
    dbus
    nix
    nodejs_20  # For Playwright-based web filtering tests
    playwright  # Playwright library
    playwright-driver.browsers  # Pre-packaged browser binaries
    openssl  # For SSL certificate generation
  ];

  # Enable SSL/TLS interception for HTTPS filtering
  services.dots-family.sslIntercept = {
    enable = true;
    countryCode = "US";
    state = "California";
    locality = "San Francisco";
    organization = "DOTS Family Mode";
  };

  # Add DOTS Family CA certificate to Firefox
  programs.firefox = {
    enable = true;
    policies = {
      Certificates = {
        Install = [
          "/var/lib/dots-family/ssl/ca.crt"
        ];
      };
    };
  };

  # Networking
  networking = {
    hostName = "dots-family-test";
    networkmanager.enable = true;
    firewall.enable = false;  # Disabled for testing
  };

  # Allow sudo without password for wheel users (development)
  security.sudo.wheelNeedsPassword = false;

  # System state version
  system.stateVersion = "24.05";
}