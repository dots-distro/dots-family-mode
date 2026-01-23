{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  
  # Fallback package builder when packages aren't available in pkgs
  buildDotsPackage = pname: args:
    pkgs.rustPlatform.buildRustPackage ({
      inherit pname;
      version = "0.1.0";
      
      src = ../../.;  # Project root
      
      cargoLock = {
        lockFile = ../../Cargo.lock;
      };
      
      # Common native build inputs
      nativeBuildInputs = with pkgs; [
        pkg-config
        makeWrapper
      ];
      
      # Common build inputs for all packages
      buildInputs = with pkgs; [
        openssl
        sqlite
        sqlx-cli
      ] ++ lib.optionals stdenv.isLinux [
        systemd  # For systemd integration
      ];
      
      # Disable tests for production builds (run in CI instead)
      doCheck = false;
      
      # Common meta
      meta = {
        description = "DOTS Family Mode ${pname}";
        homepage = "https://github.com/dots-distro/dots-family-mode";
        license = lib.licenses.mit;
        platforms = lib.platforms.linux;
      };
    } // args);
  
  # Package definitions with fallbacks
  defaultDotsPackages = {
    daemon = pkgs.dots-family-daemon or (buildDotsPackage "dots-family-daemon" {
      # eBPF dependencies for daemon
      buildInputs = (buildDotsPackage "dummy" {}).buildInputs ++ (with pkgs; [
        libbpf
        llvm
        clang
      ]);
      
      # Enable eBPF features
      cargoBuildFlags = [ "--features ebpf" ];
      
      postInstall = ''
        # Install eBPF programs
        mkdir -p $out/lib/dots-family
        cp target/bpfel-unknown-none/release/*.o $out/lib/dots-family/ || true
      '';
    });
    
    monitor = pkgs.dots-family-monitor or (buildDotsPackage "dots-family-monitor" {
      # Wayland dependencies for monitoring
      buildInputs = (buildDotsPackage "dummy" {}).buildInputs ++ (with pkgs; [
        wayland
        wayland-protocols
      ]);
    });
    
    ctl = pkgs.dots-family-ctl or (buildDotsPackage "dots-family-ctl" {});
    
    terminal-filter = pkgs.dots-terminal-filter or (buildDotsPackage "dots-terminal-filter" {});
  };
  
in {
  imports = [
    ./daemon.nix
    ./dbus.nix
    ./security.nix
    ./user-services.nix
    ./ssl-intercept.nix
  ];

  options.services.dots-family = {
    enable = lib.mkEnableOption "DOTS Family Mode parental controls";

    # Package options - users can override if needed
    package = lib.mkOption {
      type = lib.types.package;
      default = defaultDotsPackages.daemon;
      description = "The dots-family-daemon package to use";
    };

    monitorPackage = lib.mkOption {
      type = lib.types.package;
      default = defaultDotsPackages.monitor;
      description = "The dots-family-monitor package to use";
    };

    ctlPackage = lib.mkOption {
      type = lib.types.package;
      default = defaultDotsPackages.ctl;
      description = "The dots-family-ctl package to use";
    };

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

    parentUsers = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "List of users who have parent privileges";
    };

    childUsers = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "List of users who are subject to parental controls";
    };

    enableWebFiltering = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable web content filtering";
    };

    enableTerminalFiltering = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable terminal command filtering (experimental)";
    };

    enableNotifications = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable desktop notifications for policy violations and alerts";
    };

    runAsRoot = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        If true, daemon runs as root user with full privileges.
        If false (default), daemon runs as dedicated 'dots-family' user with capabilities.
        Set to true for manual systemd service installation to match root privileges.
      '';
    };

    profiles = lib.mkOption {
      type = lib.types.attrsOf (lib.types.submodule {
        options = {
          name = lib.mkOption {
            type = lib.types.str;
            description = "Display name for this profile";
          };
          
          ageGroup = lib.mkOption {
            type = lib.types.enum [ "5-7" "8-12" "13-17" "custom" ];
            default = "8-12";
            description = "Age group determines default restrictions";
          };
          
          dailyScreenTimeLimit = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            example = "2h30m";
            description = "Daily screen time limit (e.g., '2h30m', '90m')";
          };
          
          timeWindows = lib.mkOption {
            type = lib.types.listOf (lib.types.submodule {
              options = {
                start = lib.mkOption {
                  type = lib.types.str;
                  example = "09:00";
                  description = "Start time (24-hour format)";
                };
                end = lib.mkOption {
                  type = lib.types.str;
                  example = "17:00";
                  description = "End time (24-hour format)";
                };
                days = lib.mkOption {
                  type = lib.types.listOf (lib.types.enum [ "mon" "tue" "wed" "thu" "fri" "sat" "sun" ]);
                  default = [ "mon" "tue" "wed" "thu" "fri" ];
                  description = "Days when this time window applies";
    };
    
    # SSL/TLS interception options
    sslIntercept = lib.mkOption {
      type = lib.types.submodule {
        options = {
          enable = lib.mkEnableOption "Enable SSL/TLS interception for HTTPS filtering";

          certPath = lib.mkOption {
            type = lib.types.str;
            default = "/var/lib/dots-family/ssl";
            description = "Directory to store SSL certificates and keys";
          };

          caCertPath = lib.mkOption {
            type = lib.types.str;
            default = "/var/lib/dots-family/ssl/ca.crt";
            description = "Path to CA certificate";
          };

          caKeyPath = lib.mkOption {
            type = lib.types.str;
            default = "/var/lib/dots-family/ssl/ca.key";
            description = "Path to CA private key";
          };

          countryCode = lib.mkOption {
            type = lib.types.str;
            default = "US";
            description = "Country code for CA certificate";
          };

          state = lib.mkOption {
            type = lib.types.str;
            default = "California";
            description = "State for CA certificate";
          };

          locality = lib.mkOption {
            type = lib.types.str;
            default = "San Francisco";
            description = "Locality (city) for CA certificate";
          };

          organization = lib.mkOption {
            type = lib.types.str;
            default = "DOTS Family Mode";
            description = "Organization for CA certificate";
          };

          pkcs12Password = lib.mkOption {
            type = lib.types.str;
            default = "dots-family";
            description = "Password for PKCS12 certificate bundle";
          };
        };
      };
      default = {};
      description = "SSL/TLS interception configuration for HTTPS filtering";
    };
  };

  config = lib.mkIf cfg.enable {
    # Enable SSL intercept when web filtering is enabled
    services.dots-family.sslIntercept.enable = cfg.enableWebFiltering;
    
    # System packages - use the configured packages
    environment.systemPackages = [
      cfg.package      # daemon
      cfg.ctlPackage   # CLI tool
      # Monitor package is installed per-user in user-services.nix
    ];

    # State directory for database and logs
    systemd.tmpfiles.rules = [
      "d /var/lib/dots-family 0755 root root"
      "d /var/log/dots-family 0755 root root"
    ] ++ lib.optionals (cfg.enableWebFiltering && cfg.sslIntercept.enable) [
      "d ${cfg.sslIntercept.certPath} 700 root root"
    ];

    # Pass configuration to submodules using the configured packages
    services.dots-family.internal = {
      packages = {
        daemon = cfg.package;
        monitor = cfg.monitorPackage;
        ctl = cfg.ctlPackage;
      };
      config = cfg;
      inherit (cfg) runAsRoot;
    };
    

    
    # User groups for family mode
    users.groups = {
      dots-family-parents = { };
      dots-family-children = { };
    };

    # Add users to appropriate groups
    users.users = lib.mkMerge [
      (lib.genAttrs cfg.parentUsers (user: {
        extraGroups = [ "dots-family-parents" ];
      }))
      (lib.genAttrs cfg.childUsers (user: {
        extraGroups = [ "dots-family-children" ];
      }))
    ];
    
    # SSL/TLS interception configuration (when web filtering enabled)

    # Generate CA certificate on activation (when web filtering enabled)
    systemd.services.dots-family-ssl-ca = lib.mkIf (cfg.enableWebFiltering && cfg.sslIntercept.enable) {
      description = "Generate DOTS Family Mode SSL CA Certificate";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = "yes";
        ExecStart = let
          certGenerationScript = pkgs.writeScriptBin "generate-dots-family-ca" ''
            #!/bin/bash
            set -euo pipefail

            CERT_DIR="${cfg.sslIntercept.certPath}"
            CA_KEY="$CERT_DIR/ca.key"
            CA_CERT="$CERT_DIR/ca.crt"
            CA_PEM="$CERT_DIR/ca.pem"

            echo "Generating DOTS Family Mode CA certificate..."

            # Create certificate directory
            mkdir -p "$CERT_DIR"
            chmod 700 "$CERT_DIR"

            # Generate CA private key
            openssl genpkey -algorithm RSA -out "$CA_KEY" 4096

            # Generate CA certificate
            openssl req -new -x509 -key "$CA_KEY" -out "$CA_CERT" -days 3650 \
              -subj "/C=${cfg.sslIntercept.countryCode}/ST=${cfg.sslIntercept.state}/L=${cfg.sslIntercept.locality}/O=${cfg.sslIntercept.organization}/CN=DOTS Family Mode CA"

            # Create PEM bundle
            cat "$CA_CERT" "$CA_KEY" > "$CA_PEM"

            echo "CA certificate generated successfully!"
            echo "Certificate: $CA_CERT"
            echo "Private key: $CA_KEY"
          '';
    ];
  };
}