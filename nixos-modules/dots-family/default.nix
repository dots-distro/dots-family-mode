{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  
  # Build packages directly using the Rust build system
  # This follows the pattern used by most Rust projects in nixpkgs
  buildDotsPackage = { pname, cargoRoot ? "crates/${pname}" }:
    pkgs.rustPlatform.buildRustPackage {
      inherit pname;
      version = "0.1.0";
      
      src = lib.cleanSource ./../..;  # Points to project root
      
      cargoLock = {
        lockFile = ./../../Cargo.lock;
      };
      
      buildAndTestSubdir = cargoRoot;
      
      nativeBuildInputs = with pkgs; [ pkg-config ];
      buildInputs = with pkgs; [ 
        openssl sqlite sqlcipher dbus gtk4 libadwaita
        # Runtime dependencies
        procps util-linux libnotify polkit
      ];
      
      # Disable SQLx compile-time checks for Nix build
      SQLX_OFFLINE = "true";
      
      # Skip tests for individual packages (run them in workspace)
      doCheck = false;
      
      meta = with lib; {
        description = "${pname} component for DOTS Family Mode";
        license = licenses.agpl3Plus;
        maintainers = [ ];
      };
    };

  # Build the packages we need
  dotsFamilyPackages = {
    daemon = buildDotsPackage { pname = "dots-family-daemon"; };
    monitor = buildDotsPackage { pname = "dots-family-monitor"; };
    ctl = buildDotsPackage { pname = "dots-family-ctl"; };
    # Note: These would need additional dependencies
    # filter = buildDotsPackage { pname = "dots-family-filter"; };
    # gui = buildDotsPackage { pname = "dots-family-gui"; };
  };
  
in {
  imports = [
    ./daemon.nix
    ./dbus.nix
    ./security.nix
    ./user-services.nix
  ];

  options.services.dots-family = {
    enable = lib.mkEnableOption "DOTS Family Mode parental controls";

    # Package options - users can override if needed
    package = lib.mkOption {
      type = lib.types.package;
      default = dotsFamilyPackages.daemon;
      description = "The dots-family-daemon package to use";
    };

    monitorPackage = lib.mkOption {
      type = lib.types.package;
      default = dotsFamilyPackages.monitor;
      description = "The dots-family-monitor package to use";
    };

    ctlPackage = lib.mkOption {
      type = lib.types.package;
      default = dotsFamilyPackages.ctl;
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
              };
            });
            default = [ ];
            description = "Time windows when computer access is allowed";
          };
          
          allowedApplications = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            example = [ "firefox" "inkscape" "tuxmath" ];
            description = "List of allowed application IDs";
          };
          
          blockedApplications = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            example = [ "discord" "steam" ];
            description = "List of blocked application IDs";
          };
          
          webFilteringLevel = lib.mkOption {
            type = lib.types.enum [ "strict" "moderate" "minimal" "disabled" ];
            default = "moderate";
            description = "Web content filtering level";
          };
        };
      });
      default = { };
      description = "User profiles with their restrictions";
    };

    settings = lib.mkOption {
      type = lib.types.attrs;
      default = { };
      description = "Additional configuration settings";
    };
  };

  config = lib.mkIf cfg.enable {
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
    ];

    # Pass configuration to submodules using the configured packages
    services.dots-family.internal = {
      packages = {
        daemon = cfg.package;
        monitor = cfg.monitorPackage;
        ctl = cfg.ctlPackage;
      };
      config = cfg;
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
  };
}