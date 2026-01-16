{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  
in {
  config = lib.mkIf cfg.enable {
    # Polkit rules for parent authentication
    security.polkit.extraConfig = ''
      // DOTS Family Mode - Parent Authentication Rules
      
      // Allow parent users to manage family controls
      polkit.addRule(function(action, subject) {
        if (action.id == "org.dots.family.manage" ||
            action.id.indexOf("org.dots.family.") === 0) {
          
          // Check if user is in parent group
          var parentUsers = [${lib.concatMapStringsSep ", " (user: ''"${user}"'') cfg.parentUsers}];
          if (parentUsers.indexOf(subject.user) !== -1) {
            return polkit.Result.YES;
          }
          
          // For other users, require authentication with admin password
          return polkit.Result.AUTH_ADMIN;
        }
      });

      // Allow child users limited operations without authentication
      polkit.addRule(function(action, subject) {
        if (action.id == "org.dots.family.view-status" ||
            action.id == "org.dots.family.request-exception") {
          
          var childUsers = [${lib.concatMapStringsSep ", " (user: ''"${user}"'') cfg.childUsers}];
          if (childUsers.indexOf(subject.user) !== -1) {
            return polkit.Result.YES;
          }
        }
      });

      // Prevent children from bypassing controls
      polkit.addRule(function(action, subject) {
        if (action.id.indexOf("org.freedesktop.systemd1.manage-units") === 0) {
          // Check if trying to manage DOTS Family services
          if (action.lookup("unit").indexOf("dots-family") !== -1) {
            var childUsers = [${lib.concatMapStringsSep ", " (user: ''"${user}"'') cfg.childUsers}];
            if (childUsers.indexOf(subject.user) !== -1) {
              return polkit.Result.NO;
            }
          }
        }
      });
    '';

    # Create polkit actions file
    environment.etc."polkit-1/actions/org.dots.family.policy".text = ''
      <?xml version="1.0" encoding="UTF-8"?>
      <!DOCTYPE policyconfig PUBLIC
        "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/PolicyKit/1.0/policyconfig.dtd">
      <policyconfig>
        <vendor>DOTS Project</vendor>
        <vendor_url>https://github.com/dots-family</vendor_url>
        
        <action id="org.dots.family.manage">
          <description>Manage DOTS Family Mode controls</description>
          <description xml:lang="en">Manage parental control settings, profiles, and policies</description>
          <message>Authentication is required to manage family controls</message>
          <defaults>
            <allow_any>no</allow_any>
            <allow_inactive>no</allow_inactive>
            <allow_active>auth_admin</allow_active>
          </defaults>
        </action>
        
        <action id="org.dots.family.view-status">
          <description>View family mode status</description>
          <description xml:lang="en">View current activity and restrictions</description>
          <message>View family control status</message>
          <defaults>
            <allow_any>no</allow_any>
            <allow_inactive>no</allow_inactive>
            <allow_active>yes</allow_active>
          </defaults>
        </action>
        
        <action id="org.dots.family.request-exception">
          <description>Request temporary exception</description>
          <description xml:lang="en">Request temporary access to blocked content or extended time</description>
          <message>Request exception to family controls</message>
          <defaults>
            <allow_any>no</allow_any>
            <allow_inactive>no</allow_inactive>
            <allow_active>yes</allow_active>
          </defaults>
        </action>
        
        <action id="org.dots.family.update-database">
          <description>Update family control database</description>
          <description xml:lang="en">Modify database containing profiles and policies</description>
          <message>Authentication is required to modify family control data</message>
          <defaults>
            <allow_any>no</allow_any>
            <allow_inactive>no</allow_inactive>
            <allow_active>auth_admin</allow_active>
          </defaults>
        </action>
      </policyconfig>
    '';

    # Enhanced security settings
    security.sudo.extraRules = lib.mkIf (cfg.parentUsers != [ ]) [
      # Allow parent users to manage family services without password
      {
        users = cfg.parentUsers;
        commands = [
          {
            command = "${pkgs.systemd}/bin/systemctl start dots-family-daemon.service";
            options = [ "NOPASSWD" ];
          }
          {
            command = "${pkgs.systemd}/bin/systemctl stop dots-family-daemon.service";
            options = [ "NOPASSWD" ];
          }
          {
            command = "${pkgs.systemd}/bin/systemctl restart dots-family-daemon.service";
            options = [ "NOPASSWD" ];
          }
          {
            command = "${pkgs.systemd}/bin/systemctl status dots-family-daemon.service";
            options = [ "NOPASSWD" ];
          }
          {
            command = "${cfg.internal.packages.ctl}/bin/dots-family-ctl *";
            options = [ "NOPASSWD" ];
          }
        ];
      }
    ];

    # Prevent children from accessing daemon configuration
    systemd.tmpfiles.rules = [
      # Secure daemon configuration
      "Z /var/lib/dots-family 0750 dots-family dots-family-parents"
      "Z /var/log/dots-family 0750 dots-family dots-family-parents"
      
      # Prevent children from accessing config files
      "z /var/lib/dots-family/daemon.toml 0640 dots-family dots-family-parents"
      "z /var/lib/dots-family/*.db* 0640 dots-family dots-family-parents"
    ];

    # Optional AppArmor profile for enhanced security
    security.apparmor = lib.mkIf config.security.apparmor.enable {
      policies = {
        "dots-family-daemon" = {
          enable = true;
          profile = ''
            #include <tunables/global>
            
            ${cfg.internal.packages.daemon}/bin/dots-family-daemon {
              #include <abstractions/base>
              #include <abstractions/dbus-session-strict>
              #include <abstractions/nameservice>
              
              # Allow daemon to read its configuration
              /var/lib/dots-family/ r,
              /var/lib/dots-family/** rw,
              
              # Allow logging
              /var/log/dots-family/ r,
              /var/log/dots-family/** rw,
              
              # Allow network access for filter updates
              network inet stream,
              network inet dgram,
              
              # Deny access to other users' home directories
              deny /home/*/  r,
              deny /home/*/** rw,
              
              # Deny access to sensitive system files
              deny /etc/passwd r,
              deny /etc/shadow r,
              deny /etc/sudoers* r,
              
              # Allow reading system information
              /proc/*/stat r,
              /proc/*/cmdline r,
              /sys/class/net/*/statistics/* r,
              
              # Temporary directory access
              /tmp/ r,
              owner /tmp/dots-family-** rw,
              
              # Binary execution
              ${cfg.internal.packages.daemon}/bin/dots-family-daemon ix,
            }
          '';
        };
      };
    };

    # File system restrictions for child users
    environment.etc."security/limits.d/dots-family.conf".text = ''
      # DOTS Family Mode - Resource limits for child users
      ${lib.concatMapStringsSep "\n" (user: ''
        # Limit processes and memory for ${user}
        ${user} hard nproc 100
        ${user} hard as 2097152  # 2GB memory limit
        ${user} hard fsize 1048576  # 1GB file size limit
      '') cfg.childUsers}
    '';

    # Network security - prevent children from bypassing proxy
    networking.firewall = lib.mkIf cfg.enableWebFiltering {
      extraCommands = ''
        # DOTS Family Mode - Block direct internet access for children
        # Force all HTTP/HTTPS through proxy
        ${lib.concatMapStringsSep "\n" (user: ''
          # Block direct access for ${user} (will be enforced by monitor)
          # This is a placeholder - actual enforcement happens in the monitor
        '') cfg.childUsers}
      '';
    };
  };
}