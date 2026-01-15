{ config, pkgs, ... }:

{
  # Boot configuration
  boot.loader.grub.enable = true;
  boot.loader.grub.device = "/dev/vda";

  # Networking
  networking.hostName = "dots-family-test";
  networking.networkmanager.enable = true;
  networking.firewall.enable = false;  # Disable for testing

  # Users
  users.users.root.password = "root";
  users.users.test = {
    isNormalUser = true;
    password = "test";
    extraGroups = [ "wheel" "networkmanager" ];
  };

  # SSH for access
  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "yes";
    settings.PasswordAuthentication = true;
  };

  # Basic packages
  environment.systemPackages = with pkgs; [
    vim
    git
    cargo
    rustc
    sqlite
    pkg-config
    dbus
    systemd
    busctl
    firefox
    foot
    htop
  ];

  # Desktop environment
  services.xserver = {
    enable = true;
    displayManager.lightdm.enable = true;
    windowManager.i3.enable = true;
  };

  # Development tools
  nix.settings.experimental-features = [ "nix-command" "flakes" ];
  
  # System state version
  system.stateVersion = "24.05";
}
