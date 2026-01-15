# NixOS VM configuration for testing DOTS Family Mode
{ config, pkgs, lib, ... }:

let
  # Build DOTS Family Mode from local source
  dots-family-mode = pkgs.rustPlatform.buildRustPackage {
    pname = "dots-family-mode";
    version = "0.1.0";
    
    src = ./.;
    
    cargoLock = {
      lockFile = ./Cargo.lock;
    };
    
    nativeBuildInputs = with pkgs; [
      pkg-config
      sqlite
    ];
    
    buildInputs = with pkgs; [
      sqlite
      dbus
      systemd
    ];
    
    # Build all binaries
    cargoBuildFlags = [ "--workspace" ];
    
    # Install all binaries
    postInstall = ''
      # Install systemd service
      mkdir -p $out/lib/systemd/system
      cp systemd/dots-family-daemon.service $out/lib/systemd/system/
      
      # Install DBus service
      mkdir -p $out/share/dbus-1/system-services
      cp dbus/org.dots.FamilyDaemon.service $out/share/dbus-1/system-services/
      
      # Install DBus policy
      mkdir -p $out/share/dbus-1/system.d
      cat > $out/share/dbus-1/system.d/org.dots.FamilyDaemon.conf << EOF
<!DOCTYPE busconfig PUBLIC
 "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy context="default">
    <allow own="org.dots.FamilyDaemon"/>
    <allow send_destination="org.dots.FamilyDaemon"/>
    <allow receive_sender="org.dots.FamilyDaemon"/>
  </policy>
  
  <policy user="root">
    <allow own="org.dots.FamilyDaemon"/>
    <allow send_destination="org.dots.FamilyDaemon"/>
    <allow receive_sender="org.dots.FamilyDaemon"/>
  </policy>
  
  <policy group="wheel">
    <allow send_destination="org.dots.FamilyDaemon"/>
    <allow receive_sender="org.dots.FamilyDaemon"/>
  </policy>
</busconfig>
EOF
    '';
    
    meta = with lib; {
      description = "DOTS Family Mode - Parental control system for NixOS";
      license = licenses.agpl3Plus;
    };
  };

in {
  # VM configuration
  virtualisation = {
    memorySize = 4096;
    cores = 4;
    diskSize = 8192;
    
    # Enable graphics for testing GUI applications
    graphics = true;
    
    # Forward ports for debugging
    forwardPorts = [
      { from = "host"; host.port = 2222; guest.port = 22; }
    ];
  };
  
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
  
  # Enable SSH for remote access
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
    };
  };
  
  # Desktop environment for testing
  services.xserver = {
    enable = true;
    displayManager.lightdm.enable = true;
    desktopManager.xfce.enable = true;
  };
  
  # Wayland compositor (Niri) for testing
  programs.niri.enable = true;
  
  # Install required packages
  environment.systemPackages = with pkgs; [
    dots-family-mode
    firefox
    chromium
    foot
    alacritty
    sqlite
    busctl  # For DBus debugging
    htop
    vim
    git
    curl
  ];
  
  # DOTS Family Mode system service
  systemd.services.dots-family-daemon = {
    description = "DOTS Family Mode Daemon";
    after = [ "network.target" "dbus.service" ];
    wants = [ "dbus.service" ];
    wantedBy = [ "multi-user.target" ];
    
    serviceConfig = {
      Type = "dbus";
      BusName = "org.dots.FamilyDaemon";
      ExecStart = "${dots-family-mode}/bin/dots-family-daemon";
      Restart = "on-failure";
      RestartSec = 5;
      
      User = "root";
      Group = "root";
      
      StateDirectory = "dots-family";
      ConfigurationDirectory = "dots-family";
      CacheDirectory = "dots-family";
      
      ProtectSystem = "strict";
      ProtectHome = true;
      PrivateTmp = true;
      NoNewPrivileges = true;
      
      ReadWritePaths = [ "/var/lib/dots-family" "/etc/dots-family" ];
    };
    
    environment = {
      RUST_LOG = "info";
      DATABASE_URL = "sqlite:/var/lib/dots-family/family.db";
    };
  };
  
  # DBus configuration
  services.dbus = {
    enable = true;
    packages = [ dots-family-mode ];
  };
  
  # Create default configuration
  system.activationScripts.dots-family-setup = ''
    # Ensure directories exist
    mkdir -p /var/lib/dots-family
    mkdir -p /etc/dots-family
    
    # Create default daemon config if it doesn't exist
    if [ ! -f /etc/dots-family/daemon.toml ]; then
      cat > /etc/dots-family/daemon.toml << EOF
[database]
path = "/var/lib/dots-family/family.db"
encryption_key_file = "/var/lib/dots-family/encryption.key"

[policy_enforcement]
enabled = true
check_interval_seconds = 30

[logging]
level = "info"
file_path = "/var/log/dots-family/daemon.log"
EOF
    fi
    
    # Set permissions
    chown -R root:root /var/lib/dots-family
    chmod 700 /var/lib/dots-family
    chown -R root:root /etc/dots-family
    chmod 755 /etc/dots-family
  '';
  
  # Networking
  networking = {
    hostName = "dots-family-test";
    networkmanager.enable = true;
    firewall.enable = true;
  };
  
  # Development tools
  programs.vim.defaultEditor = true;
  
  # Allow sudo without password for wheel users (development)
  security.sudo.wheelNeedsPassword = false;
}