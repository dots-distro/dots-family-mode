# DOTS Family Mode - NixOS Integration

## Overview

This document describes the NixOS module structure for deploying DOTS Family Mode as a hardened system service.

## Module Structure

```
nixos-modules/
├── dots-family/
│   ├── default.nix          # Main module
│   ├── daemon.nix            # System service configuration
│   ├── dbus.nix              # DBus policies
│   ├── security.nix          # Polkit, capabilities
│   └── user-services.nix     # Per-user monitor agents
```

## Main Module (default.nix)

```nix
{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  dotsFamilyDaemon = pkgs.callPackage ../../../default.nix {};
in {
  imports = [
    ./daemon.nix
    ./dbus.nix
    ./security.nix
    ./user-services.nix
  ];

  options.services.dots-family = {
    enable = lib.mkEnableOption "DOTS Family Mode parental controls";

    databasePath = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/dots-family/family.db";
      description = "Path to SQLite database (will be encrypted)";
    };

    reportingOnly = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        If true, daemon logs activity but does not enforce limits.
        Recommended for initial deployment to verify functionality.
      '';
    };

    tailscaleExempt = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        If true, Tailscale traffic bypasses filtering.
        Critical for remote administration.
      '';
    };

    profiles = lib.mkOption {
      type = lib.types.attrsOf (lib.types.submodule {
        options = {
          dailyLimitMinutes = lib.mkOption {
            type = lib.types.int;
            description = "Daily screen time limit in minutes";
          };

          allowedApplications = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [];
            description = "Allowed application IDs (allowlist mode)";
          };

          blockedApplications = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [];
            description = "Blocked application IDs (blocklist mode)";
          };
        };
      });
      default = {};
      description = "Child profiles to configure";
    };
  };

  config = lib.mkIf cfg.enable {
    # Assertions for safe deployment
    assertions = [
      {
        assertion = cfg.reportingOnly || cfg.tailscaleExempt;
        message = ''
          DANGER: You are enabling enforcement without Tailscale exemption.
          This could lock you out of remote administration.
          Set reportingOnly = true or tailscaleExempt = true.
        '';
      }
    ];

    # Create dedicated system user
    users.users.dots-family = {
      isSystemUser = true;
      group = "dots-family";
      home = "/var/lib/dots-family";
      createHome = true;
      shell = pkgs.shadow;
      description = "DOTS Family Mode daemon user";
    };

    users.groups.dots-family = {};

    # Ensure database directory exists with correct permissions
    systemd.tmpfiles.rules = [
      "d /var/lib/dots-family 0750 dots-family dots-family -"
      "d /var/lib/dots-family/logs 0750 dots-family dots-family -"
    ];
  };
}
```

## System Service (daemon.nix)

```nix
{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  dotsFamilyDaemon = pkgs.callPackage ../../../default.nix {};

  daemonConfig = pkgs.writeText "dots-family-daemon.toml" ''
    [database]
    path = "${cfg.databasePath}"

    [enforcement]
    enabled = ${if cfg.reportingOnly then "false" else "true"}
    reporting_only = ${if cfg.reportingOnly then "true" else "false"}

    [monitoring]
    heartbeat_timeout_seconds = 30
    fail_closed = true

    [auth]
    # Parent password hash will be set via CLI
  '';
in {
  config = lib.mkIf cfg.enable {
    systemd.services.dots-family-daemon = {
      description = "DOTS Family Mode Controller";
      documentation = [ "https://github.com/yourusername/dots-family-mode" ];

      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" "dbus.service" ];
      requires = [ "dbus.service" ];

      serviceConfig = {
        Type = "dbus";
        BusName = "org.dots.FamilyDaemon";
        ExecStart = "${dotsFamilyDaemon}/bin/dots-family-daemon --config ${daemonConfig}";

        # User/Group
        User = "dots-family";
        Group = "dots-family";

        # Restart behavior
        Restart = "always";
        RestartSec = "5s";

        # Security Capabilities
        # CAP_SYS_PTRACE: Monitor child processes for tampering
        AmbientCapabilities = [ "CAP_SYS_PTRACE" ];
        CapabilityBoundingSet = [ "CAP_SYS_PTRACE" ];

        # Filesystem Isolation
        ProtectSystem = "strict";
        ProtectHome = true;  # Daemon cannot see /home
        ReadWritePaths = [
          "/var/lib/dots-family"
          "/run/dots-family"  # For PID files, sockets
        ];
        PrivateTmp = true;

        # Process Isolation
        NoNewPrivileges = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictAddressFamilies = [ "AF_UNIX" "AF_INET" "AF_INET6" ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;

        # System Call Filtering
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
          "~@resources"
        ];
        SystemCallErrorNumber = "EPERM";

        # Network (needed for future filtering)
        PrivateNetwork = false;

        # Logging
        StandardOutput = "journal";
        StandardError = "journal";
        SyslogIdentifier = "dots-family-daemon";
      };
    };

    # Runtime directory
    systemd.tmpfiles.rules = [
      "d /run/dots-family 0755 dots-family dots-family -"
    ];
  };
}
```

## Environment Variables

The daemon supports several environment variables for flexible configuration:

### Core Configuration

- **`DOTS_FAMILY_DB_PATH`**: Path to the SQLite database file
  - Default: `/var/lib/dots-family/family.db` (user config) or `/tmp/dots-family.db` (fallback)
  - Set via NixOS module: Automatically configured from `databasePath` option
  - Example: `DOTS_FAMILY_DB_PATH=/custom/path/family.db`

- **`DOTS_FAMILY_CONFIG_DIR`**: Directory containing daemon configuration files
  - Default: `~/.config/dots-family` (development) or checks environment variable
  - Set via NixOS module: `/var/lib/dots-family/config`
  - Example: `DOTS_FAMILY_CONFIG_DIR=/etc/dots-family`

### eBPF Monitoring (Optional)

If the `ebpfPackage` option is provided, these environment variables are automatically set:

- **`BPF_NETWORK_MONITOR_PATH`**: Path to network monitoring eBPF program
  - Example: `${ebpfPackage}/target/bpfel-unknown-none/release/network-monitor`

- **`BPF_FILESYSTEM_MONITOR_PATH`**: Path to filesystem monitoring eBPF program
  - Example: `${ebpfPackage}/target/bpfel-unknown-none/release/filesystem-monitor`

### NixOS Module Configuration

The NixOS module automatically configures these variables in the systemd service:

```nix
services.dots-family = {
  enable = true;
  databasePath = "/var/lib/dots-family/family.db";  # Sets DOTS_FAMILY_DB_PATH
  ebpfPackage = pkgs.dots-family-ebpf;              # Sets BPF_*_PATH variables
};
```

The service configuration in `daemon.nix` includes:

```nix
Environment = [
  "DOTS_FAMILY_CONFIG_DIR=/var/lib/dots-family/config"
  "DOTS_FAMILY_DB_PATH=${cfg.databasePath}"
  # eBPF paths added conditionally if ebpfPackage is set
];
```

## DBus Configuration (dbus.nix)

```nix
{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;

  dbusConfig = pkgs.writeTextFile {
    name = "org.dots.FamilyDaemon.conf";
    text = ''
      <!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
       "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
      <busconfig>
        <!-- Daemon ownership -->
        <policy user="dots-family">
          <allow own="org.dots.FamilyDaemon"/>
        </policy>

        <!-- Root administration -->
        <policy user="root">
          <allow own="org.dots.FamilyDaemon"/>
          <allow send_destination="org.dots.FamilyDaemon"/>
          <allow receive_sender="org.dots.FamilyDaemon"/>
        </policy>

        <!-- Regular users: read-only methods -->
        <policy context="default">
          <!-- Allow activity reporting from monitors -->
          <allow send_destination="org.dots.FamilyDaemon"
                 send_interface="org.dots.FamilyDaemon"
                 send_member="ReportActivity"/>

          <!-- Allow heartbeat from monitors -->
          <allow send_destination="org.dots.FamilyDaemon"
                 send_interface="org.dots.FamilyDaemon"
                 send_member="SendHeartbeat"/>

          <!-- Allow policy queries -->
          <allow send_destination="org.dots.FamilyDaemon"
                 send_interface="org.dots.FamilyDaemon"
                 send_member="CheckApplicationAllowed"/>

          <allow send_destination="org.dots.FamilyDaemon"
                 send_interface="org.dots.FamilyDaemon"
                 send_member="GetTimeRemaining"/>

          <!-- Deny policy modifications -->
          <deny send_destination="org.dots.FamilyDaemon"
                send_interface="org.dots.FamilyDaemon"
                send_member="UpdatePolicy"/>

          <deny send_destination="org.dots.FamilyDaemon"
                send_interface="org.dots.FamilyDaemon"
                send_member="CreateProfile"/>

          <deny send_destination="org.dots.FamilyDaemon"
                send_interface="org.dots.FamilyDaemon"
                send_member="DeleteProfile"/>

          <!-- Allow receiving signals -->
          <allow receive_sender="org.dots.FamilyDaemon"
                 receive_interface="org.dots.FamilyDaemon"
                 receive_type="signal"/>
        </policy>
      </busconfig>
    '';
    destination = "/share/dbus-1/system.d/org.dots.FamilyDaemon.conf";
  };
in {
  config = lib.mkIf cfg.enable {
    services.dbus.packages = [ dbusConfig ];
  };
}
```

## Security Policies (security.nix)

```nix
{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
in {
  config = lib.mkIf cfg.enable {
    # Polkit rules for parent administration
    security.polkit.extraConfig = ''
      // Allow wheel group (parents) to modify policies
      polkit.addRule(function(action, subject) {
        if (action.id == "org.dots.FamilyDaemon.UpdatePolicy" &&
            subject.isInGroup("wheel")) {
          return polkit.Result.YES;
        }
      });

      // Allow wheel group to create/delete profiles
      polkit.addRule(function(action, subject) {
        if ((action.id == "org.dots.FamilyDaemon.CreateProfile" ||
             action.id == "org.dots.FamilyDaemon.DeleteProfile") &&
            subject.isInGroup("wheel")) {
          return polkit.Result.YES;
        }
      });

      // Deny all users (including children) from stopping the daemon
      polkit.addRule(function(action, subject) {
        if (action.id == "org.freedesktop.systemd1.manage-units" &&
            action.lookup("unit") == "dots-family-daemon.service") {
          if (subject.isInGroup("wheel")) {
            return polkit.Result.YES;
          }
          return polkit.Result.NO;
        }
      });
    '';

    # Prevent children from using systemctl to stop services
    security.sudo.extraConfig = ''
      # Prevent non-wheel users from stopping dots-family services
      %users ALL=(ALL) ALL, !/run/current-system/sw/bin/systemctl stop dots-family*
      %users ALL=(ALL) ALL, !/run/current-system/sw/bin/systemctl disable dots-family*
      %users ALL=(ALL) ALL, !/run/current-system/sw/bin/systemctl mask dots-family*
    '';
  };
}
```

## User Services (user-services.nix)

```nix
{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  dotsFamilyMonitor = pkgs.callPackage ../../../default.nix {};
in {
  config = lib.mkIf cfg.enable {
    # Per-user monitor service
    systemd.user.services.dots-family-monitor = {
      description = "DOTS Family Mode Activity Monitor";

      wantedBy = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${dotsFamilyMonitor}/bin/dots-family-monitor";
        Restart = "always";
        RestartSec = "5s";

        # Cannot be killed by user
        # Note: This is enforced by system daemon, not by systemd
      };
    };

    # Wayland bridge service (compositor-specific)
    systemd.user.services.dots-wm-bridge = {
      description = "DOTS Family Mode Wayland Bridge";

      wantedBy = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${dotsFamilyMonitor}/bin/dots-wm-bridge";
        Restart = "always";
        RestartSec = "5s";
      };
    };
  };
}
```

## Flake Integration

```nix
# flake.nix
{
  description = "DOTS Family Mode - Parental Controls for NixOS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: {
    # NixOS module
    nixosModules.dots-family = import ./nixos-modules/dots-family;

    # Package derivation
    packages = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = pkgs.callPackage ./default.nix {};
      }
    );

    # Development shell
    devShells = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rust-bin.stable.latest.default
            sqlcipher
            pkg-config
            dbus
          ];
        };
      }
    );
  };
}
```

## Deployment to Configuration

```nix
# /etc/nixos/configuration.nix or flake config
{ config, pkgs, ... }:

{
  imports = [
    # If using flake:
    inputs.dots-family.nixosModules.dots-family
  ];

  services.dots-family = {
    enable = true;
    reportingOnly = true;  # Week 1: Just log
    tailscaleExempt = true;  # Critical for remote access

    profiles = {
      "child1" = {
        dailyLimitMinutes = 120;
        allowedApplications = [
          "firefox"
          "code"
          "org.kde.konsole"
        ];
      };
    };
  };

  # Ensure Tailscale is not affected by filtering
  services.tailscale.enable = true;
  systemd.services.tailscaled.after = [ "network-pre.target" ];
  systemd.services.tailscaled.before = [ "dots-family-daemon.service" ];
}
```

## Post-Installation Setup

```bash
# 1. Set parent password
sudo dots-family-ctl auth set-password

# 2. Verify daemon is running
systemctl status dots-family-daemon

# 3. Check logs
journalctl -u dots-family-daemon -f

# 4. Test from child account
su - child1
dots-family-ctl status

# 5. Enable enforcement after 1 week of testing
# Edit configuration.nix:
services.dots-family.reportingOnly = false;
# Then rebuild:
sudo nixos-rebuild switch
```

## Rollback Procedure

If locked out:

```bash
# Via SSH as root:
systemctl stop dots-family-daemon
nixos-rebuild switch --rollback

# Or temporarily disable:
systemctl mask dots-family-daemon
reboot
```

## Monitoring & Debugging

```bash
# Check daemon status
systemctl status dots-family-daemon

# View real-time logs
journalctl -u dots-family-daemon -f

# Check DBus connection
busctl --system tree org.dots.FamilyDaemon

# Test DBus methods
busctl --system call org.dots.FamilyDaemon \
  /org/dots/FamilyDaemon \
  org.dots.FamilyDaemon \
  GetTimeRemaining

# Check database
sudo -u dots-family sqlcipher /var/lib/dots-family/family.db
```

## Security Verification

```bash
# 1. Verify daemon runs as system service
ps aux | grep dots-family-daemon
# Should show: dots-family user, NOT root or child

# 2. Check capabilities
getpcaps $(pidof dots-family-daemon)
# Should show: cap_sys_ptrace

# 3. Verify filesystem isolation
ls -la /proc/$(pidof dots-family-daemon)/root
# Should NOT show /home

# 4. Test DBus security
# As child user:
busctl --system call org.dots.FamilyDaemon \
  /org/dots/FamilyDaemon \
  org.dots.FamilyDaemon \
  UpdatePolicy "s" "{}"
# Should DENY with permission error
```

## Next Steps

1. ⏳ Implement heartbeat mechanism in daemon (Task 6)
2. 🔲 Create lockscreen Wayland component
3. 🔲 Build NixOS module structure
4. 🔲 Test deployment on VM
5. 🔲 Week-long reporting-only trial
6. 🔲 Enable enforcement gradually
