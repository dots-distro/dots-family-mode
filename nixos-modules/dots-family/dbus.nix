{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family;
  
in {
  config = lib.mkIf cfg.enable {
    # DBus service file for daemon activation
    services.dbus.packages = [
      (pkgs.writeTextFile {
        name = "dots-family-dbus-service";
        destination = "/share/dbus-1/system-services/org.dots.FamilyDaemon.service";
        text = ''
          [D-BUS Service]
          Name=org.dots.FamilyDaemon
          Exec=${cfg.internal.packages.daemon}/bin/dots-family-daemon
          User=dots-family
          SystemdService=dots-family-daemon.service
        '';
      })
    ];

    # DBus configuration for system bus access
    environment.etc."dbus-1/system.d/org.dots.FamilyDaemon.conf".text = ''
      <busconfig>
        <!-- DOTS Family Mode Daemon Policy -->
        <policy user="dots-family">
          <!-- Allow daemon to own its service name -->
          <allow own="org.dots.FamilyDaemon"/>
          <allow send_destination="org.dots.FamilyDaemon"/>
        </policy>

        <!-- Parent users can control the daemon -->
        ${lib.concatMapStringsSep "\n" (user: ''
          <policy user="${user}">
            <!-- Full access to daemon interface -->
            <allow send_destination="org.dots.FamilyDaemon"
                   send_interface="org.dots.FamilyDaemon"/>
            <allow send_destination="org.dots.FamilyDaemon"
                   send_interface="org.freedesktop.DBus.Properties"/>
            <allow send_destination="org.dots.FamilyDaemon"
                   send_interface="org.freedesktop.DBus.Introspectable"/>
          </policy>
        '') cfg.parentUsers}

        <!-- Child users have limited access -->
        ${lib.concatMapStringsSep "\n" (user: ''
          <policy user="${user}">
            <!-- Only status checking and exception requests -->
            <allow send_destination="org.dots.FamilyDaemon"
                   send_interface="org.dots.FamilyDaemon"
                   send_member="GetUserProfile"/>
            <allow send_destination="org.dots.FamilyDaemon"
                   send_interface="org.dots.FamilyDaemon"
                   send_member="GetScreenTimeUsage"/>
            <allow send_destination="org.dots.FamilyDaemon"
                   send_interface="org.dots.FamilyDaemon"
                   send_member="RequestException"/>
            <allow send_destination="org.dots.FamilyDaemon"
                   send_interface="org.dots.FamilyDaemon"
                   send_member="RequestApproval"/>
            <deny send_destination="org.dots.FamilyDaemon"
                  send_interface="org.dots.FamilyDaemon"
                  send_member="SetUserProfile"/>
            <deny send_destination="org.dots.FamilyDaemon"
                  send_interface="org.dots.FamilyDaemon"
                  send_member="UpdatePolicy"/>
          </policy>
        '') cfg.childUsers}

        <!-- Default policy - deny all -->
        <policy context="default">
          <deny send_destination="org.dots.FamilyDaemon"/>
        </policy>
      </busconfig>
    '';

    # Session bus configuration for monitor services
    environment.etc."dbus-1/session.d/dots-family.conf".text = ''
      <busconfig>
        <!-- Monitor services on session bus -->
        <policy context="default">
          <!-- Allow monitors to register on session bus -->
          <allow own="org.dots.FamilyMonitor"/>
          <allow send_destination="org.dots.FamilyMonitor"/>
        </policy>

        <!-- Parent users can control monitors -->
        ${lib.concatMapStringsSep "\n" (user: ''
          <policy user="${user}">
            <allow send_destination="org.dots.FamilyMonitor"
                   send_interface="org.dots.FamilyMonitor"/>
            <allow send_destination="org.dots.FamilyMonitor"
                   send_interface="org.freedesktop.DBus.Properties"/>
            <allow send_destination="org.dots.FamilyMonitor"
                   send_interface="org.freedesktop.DBus.Introspectable"/>
          </policy>
        '') cfg.parentUsers}

        <!-- Child users have read-only access to their own monitor -->
        ${lib.concatMapStringsSep "\n" (user: ''
          <policy user="${user}">
            <allow send_destination="org.dots.FamilyMonitor"
                   send_interface="org.dots.FamilyMonitor"
                   send_member="GetActivityStatus"/>
            <allow send_destination="org.dots.FamilyMonitor"
                   send_interface="org.dots.FamilyMonitor"
                   send_member="GetCurrentActivity"/>
          </policy>
        '') cfg.childUsers}
      </busconfig>
    '';

    # Enable DBus if not already enabled
    services.dbus.enable = true;
  };
}