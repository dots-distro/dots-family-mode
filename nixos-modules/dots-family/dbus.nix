{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  
in {
  config = lib.mkIf cfg.enable {
    # DBus service files for daemon and monitor activation
    services.dbus.packages = [
      # System service for daemon
      (pkgs.writeTextFile {
        name = "dots-family-dbus-system";
        destination = "/share/dbus-1/system-services/org.dots.FamilyDaemon.service";
        text = ''
          [D-BUS Service]
          Name=org.dots.FamilyDaemon
          Exec=${cfg.internal.packages.daemon}/bin/dots-family-daemon
          User=dots-family
          SystemdService=dots-family-daemon.service
        '';
      })
      
      # Session service for monitor
      (pkgs.writeTextFile {
        name = "dots-family-dbus-session";
        destination = "/share/dbus-1/session-services/org.dots.FamilyMonitor.service";
        text = ''
          [D-BUS Service]
          Name=org.dots.FamilyMonitor
          Exec=${cfg.internal.packages.monitor}/bin/dots-family-monitor
        '';
      })
      
      # System bus configuration for daemon
      (pkgs.writeTextFile {
        name = "dots-family-dbus-system-config";
        destination = "/share/dbus-1/system.d/org.dots.FamilyDaemon.conf";
        text = ''
          <!DOCTYPE busconfig PUBLIC
           "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
           "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
          <busconfig>
            <!-- Allow root and dots-family user to own the daemon service -->
            <policy user="root">
              <allow own="org.dots.FamilyDaemon"/>
              <allow send_destination="org.dots.FamilyDaemon"/>
              <allow receive_sender="org.dots.FamilyDaemon"/>
              <allow send_interface="org.dots.FamilyDaemon"/>
            </policy>

            <policy user="dots-family">
              <allow own="org.dots.FamilyDaemon"/>
              <allow send_destination="org.dots.FamilyDaemon"/>
              <allow receive_sender="org.dots.FamilyDaemon"/>
              <allow send_interface="org.dots.FamilyDaemon"/>
            </policy>

            <!-- Users in the dots-family group can use most methods -->
            <policy group="dots-family">
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="get_active_profile"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="check_application_allowed"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="get_remaining_time"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="list_profiles"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="validate_session"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="request_parent_permission"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="request_command_approval"/>
              
              <!-- Receive signals from daemon -->
              <allow receive_sender="org.dots.FamilyDaemon"
                     receive_interface="org.dots.FamilyDaemon"
                     receive_member="policy_updated"/>
              <allow receive_sender="org.dots.FamilyDaemon"
                     receive_interface="org.dots.FamilyDaemon"
                     receive_member="time_limit_warning"/>
              <allow receive_sender="org.dots.FamilyDaemon"
                     receive_interface="org.dots.FamilyDaemon"
                     receive_member="tamper_detected"/>
            </policy>

            <!-- Parent users (in dots-parents group) have administrative privileges -->
            <policy group="dots-parents">
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="authenticate_parent"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="create_profile"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="set_active_profile"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="revoke_session"/>
              
              <!-- Also inherit regular user permissions -->
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="get_active_profile"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="check_application_allowed"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="get_remaining_time"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="list_profiles"/>
              <allow send_destination="org.dots.FamilyDaemon"
                     send_interface="org.dots.FamilyDaemon"
                     send_member="validate_session"/>
              
              <!-- Receive all signals -->
              <allow receive_sender="org.dots.FamilyDaemon"
                     receive_interface="org.dots.FamilyDaemon"/>
            </policy>

            <!-- Default deny policy -->
            <policy context="default">
              <deny send_destination="org.dots.FamilyDaemon"/>
              <deny receive_sender="org.dots.FamilyDaemon"/>
            </policy>
          </busconfig>
        '';
      })
      
      # Session bus configuration for monitor
      (pkgs.writeTextFile {
        name = "dots-family-dbus-session-config";
        destination = "/share/dbus-1/session.d/org.dots.FamilyMonitor.conf";
        text = ''
          <!DOCTYPE busconfig PUBLIC
           "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
           "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
          <busconfig>
            <!-- User can own their own monitor service instance -->
            <policy context="default">
              <allow own="org.dots.FamilyMonitor"/>
              <allow send_destination="org.dots.FamilyMonitor"/>
              <allow receive_sender="org.dots.FamilyMonitor"/>
              <allow send_interface="org.dots.FamilyMonitor"/>
            </policy>

            <!-- Monitor users can query monitor services -->
            <policy group="dots-family">
              <allow send_destination="org.dots.FamilyMonitor"
                     send_interface="org.dots.FamilyMonitor"
                     send_member="get_current_activity"/>
              <allow send_destination="org.dots.FamilyMonitor"
                     send_interface="org.dots.FamilyMonitor"
                     send_member="get_active_window"/>
              
              <!-- Receive activity signals -->
              <allow receive_sender="org.dots.FamilyMonitor"
                     receive_interface="org.dots.FamilyMonitor"
                     receive_member="activity_changed"/>
            </policy>

            <!-- Parent users have full access -->
            <policy group="dots-parents">
              <allow send_destination="org.dots.FamilyMonitor"
                     send_interface="org.dots.FamilyMonitor"/>
              <allow receive_sender="org.dots.FamilyMonitor"
                     receive_interface="org.dots.FamilyMonitor"/>
            </policy>
          </busconfig>
        '';
      })
    ];

    # Create required system groups and users
    users.groups = {
      dots-family = { };
      dots-parents = { };
    };

    users.users.dots-monitor = {
      isSystemUser = true;
      group = "dots-family";
      shell = pkgs.bash;
      home = "/var/empty";
      description = "DOTS Family Monitor Service";
    };

    # Enable DBus if not already enabled
    services.dbus.enable = true;
  };
}