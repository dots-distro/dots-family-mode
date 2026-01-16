# NixOS Container Configuration for DOTS Family Mode Testing
# This creates a lightweight container with full eBPF and DBus capabilities
#
# Installation:
# 1. Copy this to /etc/nixos/containers/dots-testing.nix
# 2. Add to /etc/nixos/configuration.nix: imports = [ ./containers/dots-testing.nix ];
# 3. Run: sudo nixos-rebuild switch
# 4. Start: sudo nixos-container start dots-testing
# 5. Login: sudo nixos-container login dots-testing

{ config, pkgs, ... }:

{
  containers.dots-testing = {
    autoStart = false;
    enableTun = true;
    ephemeral = true;  # Clean slate each restart
    
    # Bind mount the project directory for development
    bindMounts = {
      "/workspace" = {
        hostPath = "/home/shift/code/endpoint-agent/dots-detection/dots-familt-mode";
        isReadOnly = false;
      };
    };
    
    config = { config, pkgs, ... }: {
      system.stateVersion = "23.11";
      
      # Enable eBPF support with latest kernel
      boot.kernelPackages = pkgs.linuxPackages_latest;
      boot.kernelModules = [ "bpf" ];
      boot.kernelParams = [ "bpf.unprivileged_bpf_disabled=0" ];
      
      # Enable essential system services
      services.dbus.enable = true;
      systemd.services.dbus.wantedBy = [ "multi-user.target" ];
      
      # Complete Rust development environment
      environment.systemPackages = with pkgs; [
        # Rust toolchain
        rustc
        cargo
        rustfmt
        clippy
        
        # Build dependencies
        pkg-config
        openssl
        sqlite
        sqlcipher
        
        # eBPF development
        libbpf
        elfutils
        clang
        llvm
        bcc
        
        # System tools
        git
        htop
        strace
        tcpdump
        lsof
        ss
        procps
        util-linux
        
        # Testing tools
        curl
        firefox
        gnome.nautilus
        gedit
      ];
      
      # Test users with proper permissions
      users.users.testuser = {
        isNormalUser = true;
        extraGroups = [ "wheel" "audio" "video" ];
        password = "test123";
        shell = pkgs.bash;
      };
      
      users.users.root = {
        password = "root123";
      };
      
      # Security and capabilities for eBPF
      security.sudo.enable = true;
      security.sudo.wheelNeedsPassword = false;
      
      # Networking configuration
      networking.hostName = "dots-test-container";
      networking.firewall.enable = false;
      networking.dhcpcd.enable = false;
      networking.defaultGateway = "192.168.100.1";
      networking.interfaces.eth0.ipv4.addresses = [{
        address = "192.168.100.10";
        prefixLength = 24;
      }];
      
      # DBus policy for DOTS Family Daemon
      services.dbus.packages = [
        (pkgs.writeTextFile {
          name = "dots-family-daemon-dbus-config";
          destination = "/etc/dbus-1/system.d/org.dots.FamilyDaemon.conf";
          text = ''
            <!DOCTYPE busconfig PUBLIC
             "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
             "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
            <busconfig>
              <!-- Allow root to own the service -->
              <policy user="root">
                <allow own="org.dots.FamilyDaemon"/>
                <allow send_destination="org.dots.FamilyDaemon"/>
                <allow receive_sender="org.dots.FamilyDaemon"/>
              </policy>
              
              <!-- Allow wheel group users to use the service -->
              <policy group="wheel">
                <allow send_destination="org.dots.FamilyDaemon"/>
                <allow receive_sender="org.dots.FamilyDaemon"/>
              </policy>
              
              <!-- Allow testuser to use the service -->
              <policy user="testuser">
                <allow send_destination="org.dots.FamilyDaemon"/>
                <allow receive_sender="org.dots.FamilyDaemon"/>
              </policy>
              
              <!-- Deny all other users -->
              <policy context="default">
                <deny send_destination="org.dots.FamilyDaemon"/>
              </policy>
            </busconfig>
          '';
        })
      ];
      
      # Environment variables for testing
      environment.variables = {
        RUST_LOG = "debug";
        RUST_BACKTRACE = "1";
        DATABASE_URL = "sqlite:///tmp/dots-family-test.db";
        DOTS_TEST_MODE = "1";
      };
      
      # Systemd service for DOTS Family Daemon (optional)
      systemd.services.dots-family-daemon = {
        enable = false;  # Manual start for testing
        description = "DOTS Family Mode Daemon";
        after = [ "network.target" "dbus.service" ];
        wantedBy = [ "multi-user.target" ];
        
        serviceConfig = {
          Type = "simple";
          User = "root";
          Group = "root";
          WorkingDirectory = "/workspace";
          ExecStart = "${pkgs.cargo}/bin/cargo run -p dots-family-daemon";
          Restart = "on-failure";
          RestartSec = "5s";
          
          # Capabilities for eBPF
          AmbientCapabilities = [ "CAP_SYS_ADMIN" "CAP_NET_ADMIN" ];
          CapabilityBoundingSet = [ "CAP_SYS_ADMIN" "CAP_NET_ADMIN" ];
        };
        
        environment = {
          RUST_LOG = "debug";
          DATABASE_URL = "sqlite:///tmp/dots-family-test.db";
        };
      };
    };
  };
}