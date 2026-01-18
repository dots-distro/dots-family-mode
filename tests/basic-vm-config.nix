# Simple VM configuration for DOTS Family Mode testing
{ config, pkgs, lib, modulesPath, ... }:

{
  imports = [ "${modulesPath}/virtualisation/qemu-vm.nix" ];

  # Basic VM settings
  virtualisation.memorySize = 2048;
  virtualisation.cores = 2;

  # Users
  users.users = {
    root.password = "root";
    testuser = {
      isNormalUser = true;
      password = "test";
      extraGroups = [ "wheel" ];
    };
  };

  # Essential services
  services.dbus.enable = true;
  networking.hostName = "dots-test-vm";
  
  # Build DOTS Family Mode from source using rustPlatform
  environment.systemPackages = with pkgs; [
    # Development tools for building in VM
    rustc
    cargo
    pkg-config
    git
    
    # Runtime dependencies
    sqlite
    sqlcipher
    dbus
    systemd
    util-linux
    procps
    
    # Build DOTS binaries using a simple derivation
    (pkgs.stdenv.mkDerivation {
      name = "dots-family-mode-test";
      src = ../.; # Source from parent directory
      
      nativeBuildInputs = [ rustc cargo pkg-config ];
      buildInputs = [ sqlite sqlcipher dbus openssl ];
      
      buildPhase = ''
        # Set up cargo environment
        export CARGO_HOME=$TMPDIR/cargo
        export PATH=$CARGO_HOME/bin:$PATH
        
        # Build essential binaries only
        cargo build --release --bin dots-family-daemon
        cargo build --release --bin dots-family-ctl  
        cargo build --release --bin dots-family-monitor
      '';
      
      installPhase = ''
        mkdir -p $out/bin
        cp target/release/dots-family-daemon $out/bin/
        cp target/release/dots-family-ctl $out/bin/
        cp target/release/dots-family-monitor $out/bin/
        
        # Make binaries executable
        chmod +x $out/bin/*
      '';
      
      # Skip check phase to avoid test issues
      doCheck = false;
    })
  ];

  # Allow passwordless sudo for testing
  security.sudo.wheelNeedsPassword = false;

  system.stateVersion = "24.05";
}