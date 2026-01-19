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
        
        # Security hardening
        DynamicUser = false;  # Need specific user for database access
        User = "dots-family";
        Group = "dots-family";
        
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
      
      # Environment variables for daemon
      environment = {
        RUST_LOG = "info";
        DOTS_FAMILY_DB_PATH = cfg.databasePath;
        DOTS_FAMILY_REPORTING_ONLY = lib.boolToString cfg.reportingOnly;
        DOTS_FAMILY_ENABLE_NOTIFICATIONS = lib.boolToString cfg.enableNotifications;
      };
      
      # Configuration file setup
      preStart = ''
        # Ensure database directory exists
        mkdir -p "$(dirname "${cfg.databasePath}")"
        chown dots-family:dots-family "$(dirname "${cfg.databasePath}")"
        
        # Generate configuration file
        ${pkgs.writeShellScript "generate-config" ''
          cat > /var/lib/dots-family/daemon.toml << 'EOF'
          [auth]
          parent_password_hash = "$PARENT_PASSWORD_HASH"
          session_timeout_minutes = 60
          
          [database]
          path = "${cfg.databasePath}"
          
          ${lib.optionalString cfg.enableWebFiltering ''
          [web_filtering]
          enable = true
          proxy_port = 8888
          block_page_template = "/etc/dots-family/blocked.html"
          ''}
          
          ${lib.optionalString cfg.enableTerminalFiltering ''
          [terminal_filtering]
          enable = true
          educational_mode = true
          ''}
          
          # User profiles
          ${lib.concatMapStrings (profileName: 
            let profile = cfg.profiles.${profileName}; in ''
            [[profiles]]
            name = "${profile.name}"
            age_group = "${profile.ageGroup}"
            ${lib.optionalString (profile.dailyScreenTimeLimit != null) ''
            daily_screen_time_limit = "${profile.dailyScreenTimeLimit}"
            ''}
            allowed_applications = [${lib.concatMapStringsSep ", " (app: ''"${app}"'') profile.allowedApplications}]
            blocked_applications = [${lib.concatMapStringsSep ", " (app: ''"${app}"'') profile.blockedApplications}]
            web_filtering_level = "${profile.webFilteringLevel}"
            
            ${lib.concatMapStrings (window: ''
            [[profiles.time_windows]]
            start = "${window.start}"
            end = "${window.end}"
            days = [${lib.concatMapStringsSep ", " (day: ''"${day}"'') window.days}]
            '') profile.timeWindows}
            
            '') (builtins.attrNames cfg.profiles)}
          EOF
        ''}
      '';
    };
    
    # Create system user for daemon
    users.users.dots-family = {
      description = "DOTS Family Mode daemon user";
      isSystemUser = true;
      group = "dots-family";
      home = "/var/lib/dots-family";
      createHome = true;
    };
    
    users.groups.dots-family = { };
  };
}