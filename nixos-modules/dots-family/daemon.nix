{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  internal = cfg.internal or { };
  packages = internal.packages or { };
  
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
        ExecStart = "/run/wrappers/bin/dots-family-daemon";
        
        # User configuration - root or dedicated user
        DynamicUser = false;  # Need specific user for database access
        ${if cfg.runAsRoot then ''
        User = "root";
        Group = "root";
        '' else ''
        User = "dots-family";
        Group = "dots-family";
        ''}
        
        # Filesystem protection
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ "/var/lib/dots-family" "/var/log/dots-family" ];
        PrivateTmp = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        
        # Network restrictions
        PrivateNetwork = false;  # Need network for filter list updates
        RestrictAddressFamilies = [ "AF_UNIX" "AF_INET" "AF_INET6" ];
        
        # Capability restrictions for eBPF monitoring
        CapabilityBoundingSet = [ 
          "CAP_SYS_ADMIN"        # eBPF operations and tracepoint attachment
          "CAP_NET_ADMIN"        # Network monitoring and filtering
          "CAP_SYS_PTRACE"       # Process monitoring
          "CAP_DAC_READ_SEARCH"  # Filesystem access for monitoring
        ];
        AmbientCapabilities = [
          "CAP_SYS_ADMIN"        # eBPF operations
          "CAP_NET_ADMIN"        # Network monitoring  
          "CAP_SYS_PTRACE"       # Process monitoring
          "CAP_DAC_READ_SEARCH"  # Filesystem access
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
      
      # Only create dedicated user if not running as root
      ${lib.optionalString (!cfg.runAsRoot) ''
      # Create system user for daemon
      users.users.dots-family = {
        description = "DOTS Family Mode daemon user";
        isSystemUser = true;
        group = "dots-family";
        home = "/var/lib/dots-family";
        createHome = true;
      };
      
      users.groups.dots-family = { };
      ''}