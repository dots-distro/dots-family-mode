{ config, pkgs, lib, modulesPath, ... }:

let
  dotsPackages = pkgs.stdenv.mkDerivation {
    name = "dots-family-mode-binaries";
    src = ../.;
    
    nativeBuildInputs = with pkgs; [ rustc cargo ];
    
    buildPhase = ''
      export CARGO_HOME=$(mktemp -d)
      cargo build --release \
        --bin dots-family-daemon \
        --bin dots-family-ctl \
        --bin dots-family-monitor
    '';
    
    installPhase = ''
      mkdir -p $out/bin
      cp target/x86_64-unknown-linux-gnu/release/dots-family-daemon $out/bin/
      cp target/x86_64-unknown-linux-gnu/release/dots-family-ctl $out/bin/
      cp target/x86_64-unknown-linux-gnu/release/dots-family-monitor $out/bin/
      chmod +x $out/bin/*
    '';
  };
in
{
  imports = [ "${modulesPath}/virtualisation/qemu-vm.nix" ];

  time.timeZone = "UTC";
  i18n.defaultLocale = "en_US.UTF-8";
  
  networking.hostName = "dots-test-vm";
  networking.networkmanager.enable = true;
  networking.firewall.enable = false;

  virtualisation = {
    memorySize = 2048;  # More memory for complete system
    diskSize = 4096;    # More disk for DOTS binaries
    
    forwardPorts = [
      { from = "host"; host.port = 22221; guest.port = 22; }
    ];
    
    graphics = false;
    qemu.consoles = [ "ttyS0" ];
  };

  users.users.root.password = "root";
  users.users.test = {
    isNormalUser = true;
    password = "test";
    extraGroups = [ "wheel" "networkmanager" "dbus" ];
  };

  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "yes";
    settings.PasswordAuthentication = true;
  };

  # Essential services for DOTS
  services.dbus = {
    enable = true;
    packages = [ pkgs.dbus ];
  };

  systemd.user.services.dbus = {
    enable = true;
  };

  # Install DOTS binaries
  environment.systemPackages = with pkgs; [
    dotsPackages
    sqlite
    dbus
    systemd
    util-linux
    procps
    vim
    curl
    file
    jq
    netcat-openbsd
  ];

  security.sudo.wheelNeedsPassword = false;

  # Disable GUI services  
  services.xserver.enable = false;
  services.displayManager.enable = false;
  
  boot.initrd.systemd.enable = true;
  systemd.services.NetworkManager-wait-online.enable = false;

  system.stateVersion = "24.05";
}
