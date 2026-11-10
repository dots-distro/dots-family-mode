{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;

  # The filter binary parses a TOML file with these three tables; the key names
  # come from crates/dots-family-filter/src/config.rs.
  filterConfig = pkgs.writeText "dots-family-filter.toml" ''
    [proxy]
    bind_address = "${cfg.filter.bindAddress}"
    port = ${toString cfg.filter.port}
    upstream_timeout_seconds = 30
    max_body_size_mb = 10

    [filtering]
    enabled = true
    safe_search_enforcement = true
    block_categories = [ "adult", "violence", "gambling", "drugs" ]
    allow_categories = [ "educational", "children", "reference" ]
    blocked_domains = [ ]
    allowed_domains = [ ]
    custom_rules_enabled = true

    [daemon]
    dbus_interface = "org.dots.FamilyDaemon"
    check_permissions = true
    log_activity = true
  '';

in {
  options.services.dots-family.filter = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Start the DOTS Family Mode web filtering proxy
        (`dots-family-filter.service`).

        This module starts the proxy only. It does not redirect any traffic to
        it: no transparent proxy, iptables/nftables rule, PAC file or browser
        setting is installed. A client must be configured by hand to send HTTP
        traffic to the proxy port below for filtering to take effect.
      '';
    };

    bindAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = ''
        Address the filter proxy binds to. Keep the default loopback address
        unless clients on another host are explicitly meant to use the proxy.
      '';
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      description = ''
        Port the filter proxy listens on. Clients must be configured to use
        this address and port; no traffic is redirected to it automatically.
      '';
    };
  };

  config = lib.mkIf (cfg.enable && cfg.filter.enable) {
    systemd.services.dots-family-filter = {
      description = "DOTS Family Mode web filtering proxy";
      documentation = [ "man:dots-family-filter(1)" ];

      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.filterPackage}/bin/dots-family-filter --config-path ${filterConfig} --bind-address ${cfg.filter.bindAddress} --port ${toString cfg.filter.port}";

        # Dedicated, unprivileged identity. The proxy binds a high port and
        # needs no kernel capabilities.
        DynamicUser = true;

        # Filesystem protection
        ProtectSystem = "strict";
        ProtectHome = "read-only";
        PrivateTmp = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;

        # No capabilities at all; in particular no CAP_SYS_ADMIN.
        CapabilityBoundingSet = [ "" ];
        AmbientCapabilities = [ ];
        NoNewPrivileges = true;

        PrivateDevices = true;
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;

        # Network access for the proxy itself
        PrivateNetwork = false;
        RestrictAddressFamilies = [ "AF_INET" "AF_INET6" "AF_UNIX" ];

        # Restart policy
        Restart = "on-failure";
        RestartSec = 5;
        StartLimitBurst = 3;
        StartLimitIntervalSec = 300;

        # Logging
        StandardOutput = "journal";
        StandardError = "journal";
        SyslogIdentifier = "dots-family-filter";
      };
    };
  };
}
