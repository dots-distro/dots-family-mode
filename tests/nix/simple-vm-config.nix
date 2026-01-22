# Simple VM test for DOTS Family Mode using working build approach
{ config, pkgs, lib, ... }:

let
  # Build DOTS Family Mode using rustPlatform (which works)
  dots-family-mode = pkgs.rustPlatform.buildRustPackage {
    pname = "dots-family-mode";
    version = "0.1.0";
    
    src = ../.;
    
    cargoLock = {
      lockFile = ../Cargo.lock;
    };
    
    nativeBuildInputs = with pkgs; [
      pkg-config
    ];
    
    buildInputs = with pkgs; [
      openssl
      sqlite
      sqlcipher
      dbus
    ];
    
    # Build essential binaries only 
    cargoBuildFlags = [ "--bin" "dots-family-daemon" "--bin" "dots-family-ctl" "--bin" "dots-family-monitor" ];
    
    # Skip tests to avoid integration test issues
    doCheck = false;
    
    meta = with lib; {
      description = "DOTS Family Mode for VM testing";
    };
  };

in {
  imports = [ "${pkgs.path}/nixos/modules/virtualisation/qemu-vm.nix" ];

  # Basic VM configuration
  virtualisation.vmVariant = {
    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
  };

  # Users
  users.users = {
    root.password = "root";
    testuser = {
      isNormalUser = true;
      password = "test";
      extraGroups = [ "wheel" ];
    };
  };

  # Basic services
  services.dbus.enable = true;
  
  # Install DOTS Family Mode and dependencies
  environment.systemPackages = with pkgs; [
    dots-family-mode
    sqlite
    dbus
    systemd
    util-linux
    procps
    curl
  ];

  # DBus configuration for testing
  services.dbus.packages = [
    (pkgs.writeTextFile {
      name = "dots-family-dbus-service";
      destination = "/share/dbus-1/system-services/org.dots.FamilyDaemon.service";
      text = ''
        [D-BUS Service]
        Name=org.dots.FamilyDaemon
        Exec=${dots-family-mode}/bin/dots-family-daemon
        User=root
      '';
    })
  ];

  # Allow passwordless sudo for testing
  security.sudo.wheelNeedsPassword = false;

  system.stateVersion = "24.05";
}