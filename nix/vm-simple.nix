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

  # Enable SSH for remote access
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
    };
  };

  # Basic desktop environment for testing
  services.xserver = {
    enable = true;
    displayManager.lightdm.enable = true;
    windowManager.i3.enable = true;
  };

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
  ];

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