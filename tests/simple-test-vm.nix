# Simple VM configuration for DOTS testing 
{ config, pkgs, lib, modulesPath, ... }:

{
  imports = [ "${modulesPath}/virtualisation/qemu-vm.nix" ];

  # Basic system configuration
  time.timeZone = "UTC";
  i18n.defaultLocale = "en_US.UTF-8";
  
  # Networking
  networking.hostName = "dots-test-vm";
  networking.networkmanager.enable = true;
  networking.firewall.enable = false;

  # VM-specific configuration
  virtualisation = {
    # Configure memory and disk
    memorySize = 1024;  # Reduced for headless
    diskSize = 2048;    # Reduced for minimal system
    
    # Forward SSH port to host port 22221
    forwardPorts = [
      { from = "host"; host.port = 22221; guest.port = 22; }
    ];
    
    # Headless mode - no GUI
    graphics = false;
    qemu.consoles = [ "ttyS0" ];
  };

  # Users
  users.users.root.password = "root";
  users.users.test = {
    isNormalUser = true;
    password = "test";
    extraGroups = [ "wheel" "networkmanager" ];
    openssh.authorizedKeys.keys = [
      # Allow access with an empty key for testing
      "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC0g+Z+XYOYnTMN8V9UZy9RzWMgk1YI0nKJ/g1Y2C7pN8iJ+NwP0kp+8YmKL1XqsJ2HaEqT8A2BzB1kYQ0O3tC/vZjY1Z2K1M9WH/c7nKgJ7L+1N7Q5dF+nP2C5nJJ3O2xD7OgO0B9rJJ8u1BqsC8kJ7I1e7vPH5nN0J3rA/Lp3vOh8z9TJQ2Y6jJZ1I2VJL7Z5n0M1JYP5nE2J6U0B8Jq4B3A8J1W5O0M5J2F9U6cJ0J1L/f9T5zZ+L1/9Y0J1O2J8T0zHmQ8C2A7Y7I5N/X5K1R8Q/H3v5z+e test-vm-key"
    ];
  };

  # SSH for access
  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "yes";
    settings.PasswordAuthentication = true;
    settings.PermitEmptyPasswords = true;
  };

  # D-Bus configuration for DOTS Family Mode
  services.dbus.enable = true;
  
  # Install DOTS Family Mode D-Bus policy using services.dbus.packages
  services.dbus.packages = [
    (pkgs.writeTextDir "share/dbus-1/system.d/org.dots.FamilyDaemon.conf" ''
      <!DOCTYPE busconfig PUBLIC
       "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
       "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
      <busconfig>
        <!-- Allow root to own and communicate with the service -->
        <policy user="root">
          <allow own="org.dots.FamilyDaemon"/>
          <allow send_destination="org.dots.FamilyDaemon"/>
          <allow receive_sender="org.dots.FamilyDaemon"/>
        </policy>

        <!-- Allow any user to send messages to the daemon interface -->
        <policy context="default">
          <allow send_destination="org.dots.FamilyDaemon"
                 send_interface="org.dots.FamilyDaemon"/>
          <allow receive_sender="org.dots.FamilyDaemon"/>
        </policy>
      </busconfig>
    '')
  ];

  # Minimal packages for testing - no GUI stuff
  environment.systemPackages = with pkgs; [
    sqlite
    dbus
    systemd
    util-linux
    procps
    vim
    curl
    file
  ];

  # Allow passwordless sudo for testing
  security.sudo.wheelNeedsPassword = false;

  # Disable GUI services
  services.xserver.enable = false;
  services.displayManager.enable = false;
  services.desktopManager.plasma6.enable = false;
  services.gnome.gnome-keyring.enable = false;
  
  # Minimal boot - faster startup
  boot.initrd.systemd.enable = true;
  systemd.services.NetworkManager-wait-online.enable = false;

  system.stateVersion = "24.05";
}