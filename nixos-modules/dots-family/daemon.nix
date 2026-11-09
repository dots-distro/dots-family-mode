{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  internal = cfg.internal or { };
  packages = internal.packages or { };
  
  # eBPF program paths. These are omitted entirely when no eBPF package is
  # configured: an empty-string environment variable would be read as a path,
  # so leaving it unset lets the daemon take its degraded monitoring path.
  ebpfEnvVars = lib.optionals (packages.ebpf or null != null) [
    "BPF_PROCESS_MONITOR_PATH=${packages.ebpf}/bin/process-monitor"
    "BPF_NETWORK_MONITOR_PATH=${packages.ebpf}/bin/network-monitor"
    "BPF_FILESYSTEM_MONITOR_PATH=${packages.ebpf}/bin/filesystem-monitor"
  ];

  # Parent users must be passed to the daemon, not only written into the
  # D-Bus/group configuration. The daemon's second gate accepts only root and
  # the UIDs in `DaemonConfig.parent_users`; an empty list denies every parent,
  # so this variable has to follow `cfg.parentUsers`. The daemon parses
  # `DOTS_FAMILY_PARENT_USERS` as a comma-separated list (config.rs,
  # `parent_users_from_env`), and the environment is used when the TOML key is
  # absent. The module does not generate `daemon.toml`, so the variable is the
  # only place this value reaches the service.
  parentUsersEnv = [
    "DOTS_FAMILY_PARENT_USERS=${lib.concatStringsSep "," cfg.parentUsers}"
  ];

  daemonEnvVars = [
    "DOTS_FAMILY_CONFIG_DIR=/var/lib/dots-family/config"
    "DOTS_FAMILY_DB_PATH=${internal.config.databasePath or "/var/lib/dots-family/family.db"}"
  ] ++ parentUsersEnv ++ ebpfEnvVars;

  # Generate profile initialization script
  profileInitScript = pkgs.writeShellScript "init-profiles.sh" ''
    # Wait for daemon to be ready
    for i in {1..30}; do
      if ${pkgs.systemd}/bin/busctl status org.dots.FamilyDaemon >/dev/null 2>&1; then
        echo "Daemon is ready"
        break
      fi
      echo "Waiting for daemon... ($i/30)"
      sleep 1
    done

    # Create profiles from NixOS configuration
    ${lib.concatStringsSep "\n" (lib.mapAttrsToList (username: profile: ''
      echo "Creating profile for ${username}..."
      ${packages.ctl or cfg.ctlPackage}/bin/dots-family-ctl profile create "${profile.name}" "${profile.ageGroup}" --username "${username}" || echo "Profile ${username} may already exist"
    '') cfg.profiles)}
    
    echo "Profile initialization complete"
  '';
  
in {
  options.services.dots-family.internal = lib.mkOption {
    type = lib.types.attrs;
    internal = true;
    default = { };
  };

  config = lib.mkIf cfg.enable {
    # Profile initialization service - runs after daemon starts
    systemd.services.dots-family-init-profiles = lib.mkIf (cfg.profiles != {}) {
      description = "Initialize DOTS Family Mode profiles from configuration";
      after = [ "dots-family-daemon.service" ];
      wants = [ "dots-family-daemon.service" ];
      wantedBy = [ "multi-user.target" ];
      
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = "${profileInitScript}";
        User = "root";
      };
    };
    
    # DOTS Family Daemon - Core Service
    systemd.services.dots-family-daemon = {
      description = "DOTS Family Mode Daemon - Core parental control service";
      documentation = [ "man:dots-family-daemon(1)" ];
      
      after = [ "network.target" "dbus.service" ];
      wants = [ "dbus.service" ];
      wantedBy = [ "multi-user.target" ];

      # Start limiting belongs to the [Unit] section. Setting these inside
      # serviceConfig emits them into [Service], where systemd 258 ignores
      # them ("Unknown key ... ignoring", confirmed with systemd-analyze
      # verify), so the restart burst would silently stay at the default.
      # See the trade-off comment in serviceConfig below.
      startLimitBurst = 20;
      startLimitIntervalSec = 600;

      serviceConfig = {
        Type = "dbus";
        BusName = "org.dots.FamilyDaemon";
        ExecStart = "${packages.daemon or cfg.package}/bin/dots-family-daemon";
        Environment = daemonEnvVars;
        
        # User configuration
        DynamicUser = false;
        User = "dots-family";
        Group = "dots-family";
        
        # Filesystem protection
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ "/var/lib/dots-family" "/var/log/dots-family" ];
        StateDirectory = "dots-family";
        ConfigurationDirectory = "dots-family";
        PrivateTmp = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        
        # Network restrictions
        PrivateNetwork = false;
        RestrictAddressFamilies = [ "AF_UNIX" "AF_INET" "AF_INET6" ];
        
        # Capability restrictions for eBPF monitoring
        CapabilityBoundingSet = [ 
          "CAP_SYS_ADMIN"
          "CAP_NET_ADMIN"
          "CAP_SYS_PTRACE"
          "CAP_DAC_READ_SEARCH"
        ];
        AmbientCapabilities = [
          "CAP_SYS_ADMIN"
          "CAP_NET_ADMIN"
          "CAP_SYS_PTRACE"
          "CAP_DAC_READ_SEARCH"
        ];
        NoNewPrivileges = true;
        
        # Memory protection
        MemoryDenyWriteExecute = true;
        LockPersonality = true;
        
        # Restart policy.
        #
        # "always", not "on-failure": a clean exit must not end enforcement.
        # With "on-failure" the daemon could be stopped for good simply by
        # making it exit 0 (or by systemd reaping a normal shutdown), so the
        # child would only have to wait for the service to be down.
        #
        # Trade-off: rate limiting is deliberately loose so a transient crash
        # loop (e.g. a monitor restarting) does not leave the service
        # permanently disabled after 3 failures in 5 minutes. It is still
        # bounded, because an unbounded "always" would spin on a persistent
        # configuration error and fill the journal. If the burst is exhausted
        # systemd still gives up; noticing that requires external alerting
        # (see docs/SECURITY_REVIEW.md B1 and ranked finding 4).
        Restart = "always";
        RestartSec = 5;

        # No WatchdogSec here on purpose. The daemon does **not** call
        # sd_notify / sd_notify_barrier and does not read NOTIFY_SOCKET (it
        # uses Type=dbus, whose readiness notification is D-Bus, not a
        # watchdog ping). Adding WatchdogSec without an application-side
        # heartbeat would kill a healthy daemon on the first interval, so it
        # must wait for sd_notify support in the daemon. See
        # docs/SECURITY_REVIEW.md B1 for the residual gap.
        #
        # KillMode and TimeoutStopSec are left at their systemd defaults
        # (control-group, 90 s): the daemon handles SIGTERM/SIGINT explicitly
        # (crates/dots-family-daemon/src/daemon.rs) and stops the monitoring
        # service before exiting, and control-group kill also tears down the
        # eBPF monitor children it spawned. There is no evidence here that a
        # different value is needed, so none is added.
        
        # Logging
        StandardOutput = "journal";
        StandardError = "journal";
        SyslogIdentifier = "dots-family-daemon";
      };
    };
    
    # Create dedicated user for the daemon (not running as root)
    users.users.dots-family = {
      description = "DOTS Family Mode daemon user";
      isSystemUser = true;
      group = "dots-family";
      home = "/var/lib/dots-family";
      createHome = true;
    };
    
    users.groups.dots-family = { };
    
    # Create config subdirectory in state directory
    systemd.tmpfiles.rules = [
      "d /var/lib/dots-family/config 0750 dots-family dots-family -"
    ];
  };
}
