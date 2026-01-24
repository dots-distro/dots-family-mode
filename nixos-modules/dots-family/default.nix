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
        linuxHeaders
        llvm
        clang
        rustc
      ]);
      
      # Don't strip eBPF objects
      dontStrip = true;
      
      # Add Rust eBPF target
      RUSTFLAGS = "-C target-arch=bpf";
      
      # Enable eBPF features
      cargoBuildFlags = [ "--features ebpf" ];
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

    ebpfPackage = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = null;
      description = "The dots-family-ebpf package to use for eBPF monitoring programs";
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
              };
            });
            default = [ ];
            description = "Time windows when computer access is allowed";
          };
          
          weekendTimeWindows = lib.mkOption {
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
                  default = [ "sat" "sun" ];
                  description = "Days when this time window applies";
                };
              };
            });
            default = [ ];
            description = "Time windows for weekends when computer access is allowed";
          };
          
          holidayTimeWindows = lib.mkOption {
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
              };
            });
            default = [ ];
            description = "Time windows for holidays when computer access is allowed";
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
        ebpf = cfg.ebpfPackage;
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
  };
}