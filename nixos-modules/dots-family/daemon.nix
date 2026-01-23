{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  internal = cfg.internal or { };
  packages = internal.packages or { };
  
  # eBPF program paths - only set if ebpf package is provided
  ebpfEnvVars = if packages.ebpf or null != null then [
    "DOTS_FAMILY_CONFIG_DIR=/var/lib/dots-family/config"
    "DOTS_FAMILY_DB_PATH=${internal.config.databasePath or "/var/lib/dots-family/family.db"}"
    "BPF_NETWORK_MONITOR_PATH=${packages.ebpf}/target/bpfel-unknown-none/release/network-monitor"
    "BPF_FILESYSTEM_MONITOR_PATH=${packages.ebpf}/target/bpfel-unknown-none/release/filesystem-monitor"
  ] else [
    "DOTS_FAMILY_CONFIG_DIR=/var/lib/dots-family/config"
    "DOTS_FAMILY_DB_PATH=${internal.config.databasePath or "/var/lib/dots-family/family.db"}"
  ];
  
in {
  options.services.dots-family.internal = lib.mkOption {
    type = lib.types.attrs;
    internal = true;
    default = { };
  };

  config = lib.mkIf cfg.enable {
    # DOTS Family Daemon - Core Service
    systemd.services.dots-family-daemon = {
      description = "DOTS Family Mode Daemon - Core parental control service";
      documentation = [ "man:dots-family-daemon(1)" ];
      
      after = [ "network.target" "dbus.service" ];
      wants = [ "dbus.service" ];
      wantedBy = [ "multi-user.target" ];
      
      serviceConfig = {
        Type = "dbus";
        BusName = "org.dots.FamilyDaemon";
        ExecStart = "${packages.daemon or cfg.package}/bin/dots-family-daemon";
        Environment = ebpfEnvVars;
        
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
        
        # Restart policy
        Restart = "on-failure";
        RestartSec = 5;
        StartLimitBurst = 3;
        StartLimitIntervalSec = 300;
        
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
