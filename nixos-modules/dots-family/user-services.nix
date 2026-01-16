{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  
in {
  config = lib.mkIf cfg.enable {
    # User systemd services for monitors (per-user session)
    systemd.user.services.dots-family-monitor = {
      description = "DOTS Family Mode Monitor - Activity tracking";
      documentation = [ "man:dots-family-monitor(1)" ];
      
      after = [ "graphical-session.target" ];
      wants = [ "graphical-session.target" ];
      
      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.internal.packages.monitor}/bin/dots-family-monitor";
        
        # Security restrictions for user service
        ProtectSystem = "strict";
        ProtectHome = "read-only";
        PrivateTmp = true;
        NoNewPrivileges = true;
        
        # Allow access to Wayland/X11 for window monitoring
        Environment = [
          "WAYLAND_DISPLAY"
          "DISPLAY" 
          "XDG_SESSION_TYPE"
          "XDG_CURRENT_DESKTOP"
        ];
        
        # Restart policy
        Restart = "on-failure";
        RestartSec = 3;
        StartLimitBurst = 5;
        StartLimitIntervalSec = 300;
        
        # Logging
        StandardOutput = "journal";
        StandardError = "journal";
        SyslogIdentifier = "dots-family-monitor";
      };
      
      # Environment for monitor
      environment = {
        RUST_LOG = "info";
        DOTS_FAMILY_USER = "%i";  # Will be set to username
      };
    };

    # Per-user GUI service for parents (optional)
    systemd.user.services.dots-family-gui = lib.mkIf (builtins.hasAttr "gui" cfg.internal.packages) {
      description = "DOTS Family Mode GUI - Parent dashboard";
      documentation = [ "man:dots-family-gui(1)" ];
      
      after = [ "graphical-session.target" ];
      wants = [ "graphical-session.target" ];
      
      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.internal.packages.gui}/bin/dots-family-gui --minimized";
        
        # GUI needs more access
        ProtectSystem = "strict";
        ProtectHome = "read-only";
        PrivateTmp = true;
        
        # Desktop environment access
        Environment = [
          "DISPLAY"
          "WAYLAND_DISPLAY"
          "XDG_RUNTIME_DIR"
          "XDG_SESSION_TYPE"
          "XDG_CURRENT_DESKTOP"
          "GTK_THEME"
        ];
        
        # Auto-restart
        Restart = "on-failure";
        RestartSec = 5;
        StartLimitBurst = 3;
        StartLimitIntervalSec = 300;
        
        # Logging
        StandardOutput = "journal";
        StandardError = "journal";
        SyslogIdentifier = "dots-family-gui";
      };
    };

    # Desktop entries for easy access
    environment.etc."xdg/autostart/dots-family-monitor.desktop" = lib.mkIf (cfg.childUsers != [ ]) {
      text = ''
        [Desktop Entry]
        Type=Application
        Name=DOTS Family Monitor
        Comment=Activity monitoring for parental controls
        Exec=systemctl --user start dots-family-monitor.service
        NoDisplay=true
        X-GNOME-Autostart-Phase=Applications
        X-KDE-AutostartScript=true
        # Only start for child users
        OnlyShowIn=GNOME;KDE;XFCE;
      '';
    };

    environment.etc."xdg/autostart/dots-family-gui.desktop" = lib.mkIf (cfg.parentUsers != [ ] && builtins.hasAttr "gui" cfg.internal.packages) {
      text = ''
        [Desktop Entry]
        Type=Application
        Name=DOTS Family Dashboard
        Comment=Parental control dashboard
        Exec=${cfg.internal.packages.gui}/bin/dots-family-gui
        Icon=preferences-desktop-parental-controls
        Categories=Settings;Security;
        # Only show to parent users
        OnlyShowIn=GNOME;KDE;XFCE;
        StartupNotify=true
      '';
    };

    # Application launchers
    environment.etc."applications/dots-family-ctl.desktop" = {
      text = ''
        [Desktop Entry]
        Type=Application
        Name=DOTS Family Control
        GenericName=Parental Controls CLI
        Comment=Command-line interface for DOTS Family Mode
        Exec=${pkgs.writeShellScript "dots-family-ctl-launcher" ''
          # Launch in terminal for better user experience
          if command -v gnome-terminal >/dev/null 2>&1; then
            gnome-terminal -- ${cfg.internal.packages.ctl}/bin/dots-family-ctl "$@"
          elif command -v konsole >/dev/null 2>&1; then
            konsole -e ${cfg.internal.packages.ctl}/bin/dots-family-ctl "$@"
          elif command -v xfce4-terminal >/dev/null 2>&1; then
            xfce4-terminal -e "${cfg.internal.packages.ctl}/bin/dots-family-ctl $*"
          else
            ${cfg.internal.packages.ctl}/bin/dots-family-ctl "$@"
          fi
        ''}
        Icon=utilities-system-monitor
        Categories=System;Settings;Security;
        Terminal=false
        StartupNotify=false
      '';
    };

    # Shell aliases for easy CLI access
    environment.shellAliases = {
      family = "${cfg.internal.packages.ctl}/bin/dots-family-ctl";
      family-status = "${cfg.internal.packages.ctl}/bin/dots-family-ctl status";
      family-profile = "${cfg.internal.packages.ctl}/bin/dots-family-ctl profile";
    };

    # Bash completion for CLI
    environment.etc."bash_completion.d/dots-family-ctl".text = ''
      # DOTS Family Mode CLI completion
      _dots_family_ctl_completions() {
        local cur prev opts
        COMPREPLY=()
        cur="''${COMP_WORDS[COMP_CWORD]}"
        prev="''${COMP_WORDS[COMP_CWORD-1]}"
        
        case ''${COMP_CWORD} in
          1)
            opts="status profile exception approval check help"
            COMPREPLY=( $(compgen -W "''${opts}" -- ''${cur}) )
            return 0
            ;;
          2)
            case ''${prev} in
              profile)
                opts="list show create update delete"
                COMPREPLY=( $(compgen -W "''${opts}" -- ''${cur}) )
                return 0
                ;;
              exception)
                opts="list grant deny"
                COMPREPLY=( $(compgen -W "''${opts}" -- ''${cur}) )
                return 0
                ;;
              approval)
                opts="list approve reject"
                COMPREPLY=( $(compgen -W "''${opts}" -- ''${cur}) )
                return 0
                ;;
            esac
            ;;
        esac
      }
      
      complete -F _dots_family_ctl_completions dots-family-ctl
      complete -F _dots_family_ctl_completions family
    '';

    # User session configuration for child accounts
    programs.bash.interactiveShellInit = lib.mkIf (cfg.childUsers != [ ]) ''
      # DOTS Family Mode - Child user session setup
      case "$USER" in
        ${lib.concatMapStringsSep "|" (user: user) cfg.childUsers})
          # Start monitor if not already running
          if ! systemctl --user is-active --quiet dots-family-monitor.service 2>/dev/null; then
            systemctl --user start dots-family-monitor.service 2>/dev/null || true
          fi
          
          # Display welcome message with current restrictions
          if command -v ${cfg.internal.packages.ctl}/bin/dots-family-ctl >/dev/null 2>&1; then
            echo "Welcome! Current family mode status:"
            ${cfg.internal.packages.ctl}/bin/dots-family-ctl status 2>/dev/null || echo "  (Family controls are starting up...)"
            echo ""
          fi
          ;;
      esac
    '';

    # User directories for child accounts
    systemd.tmpfiles.rules = lib.flatten (map (user: [
      # Create user-specific monitoring directories
      "d /tmp/dots-family-${user} 0755 ${user} ${user}"
      "d /run/user/%i/dots-family 0755 ${user} ${user}"
    ]) cfg.childUsers);

    # Parent notification setup
    environment.etc."dots-family/parent-notification.sh" = lib.mkIf (cfg.parentUsers != [ ]) {
      text = ''
        #!/bin/bash
        # Script to notify parents of family mode events
        
        PARENT_USERS=(${lib.concatStringsSep " " cfg.parentUsers})
        MESSAGE="$1"
        URGENCY="''${2:-normal}"
        
        for parent in "''${PARENT_USERS[@]}"; do
          # Try to send desktop notification to parent
          if command -v notify-send >/dev/null 2>&1; then
            # Send via their session
            sudo -u "$parent" DISPLAY=:0 notify-send \
              --urgency="$URGENCY" \
              --app-name="DOTS Family Mode" \
              --icon="preferences-desktop-parental-controls" \
              "Family Controls" "$MESSAGE" 2>/dev/null || true
          fi
        done
      '';
      mode = "0755";
    };

    # Allow monitor services to auto-start
    systemd.user.targets.default.wants = lib.mkIf (cfg.childUsers != [ ]) [
      # Monitors start automatically for all users
      # "dots-family-monitor.service"  # Commented out - will use autostart instead
    ];
  };
}