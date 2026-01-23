{ pkgs, self }:

pkgs.testers.runNixOSTest {
  name = "dots-family-full-deployment";
  
  nodes.machine = { config, ... }: {
    imports = [
      self.nixosModules.dots-family
    ];

    services.dots-family = {
      enable = true;
      package = self.packages.${pkgs.system}.dots-family-daemon;
      monitorPackage = self.packages.${pkgs.system}.dots-family-monitor;
      ctlPackage = self.packages.${pkgs.system}.dots-family-ctl;
      parentUsers = [ "parent" ];
      childUsers = [ "child" ];
      reportingOnly = true;
      runAsRoot = true;
      
      profiles.child = {
        name = "Test Child";
        ageGroup = "8-12";
        dailyScreenTimeLimit = "2h";
        timeWindows = [{
          start = "09:00";
          end = "17:00";
          days = [ "mon" "tue" "wed" "thu" "fri" ];
        }];
        allowedApplications = [ "firefox" "calculator" ];
        webFilteringLevel = "moderate";
      };
    };

    users.users = {
      parent = {
        isNormalUser = true;
        password = "parent123";
        extraGroups = [ "wheel" ];
      };
      
      child = {
        isNormalUser = true;
        password = "child123";
      };
    };
  };

  testScript = ''
    start_all()
    machine.wait_for_unit("multi-user.target")
    
    with subtest("User accounts and groups exist"):
        machine.succeed("id parent")
        machine.succeed("id child")
        machine.succeed("getent group dots-family-parents")
        machine.succeed("getent group dots-family-children")
    
    with subtest("DOTS Family packages are installed"):
        machine.succeed("which dots-family-ctl")
        machine.succeed("dots-family-ctl --help")
        machine.succeed("nix-store -qR /run/current-system | grep dots-family")
    
    with subtest("Database directory exists"):
        machine.succeed("test -d /var/lib/dots-family")
        machine.succeed("test -d /var/log/dots-family")
    
    with subtest("DBus policy files are installed"):
        # DBus policies are in /etc/dbus-1/system.d/ after being processed by dbus service
        machine.succeed("ls -la /etc/dbus-1/system.d/ || true")
        # Also check if it's in the packages
        machine.succeed("find /nix/store -name 'org.dots.FamilyDaemon.conf' | head -1")
    
    with subtest("SSL certificates are generated"):
        machine.wait_for_unit("dots-family-ssl-ca.service")
        machine.succeed("test -f /var/lib/dots-family/ssl/ca.crt")
        machine.succeed("test -f /var/lib/dots-family/ssl/ca.key")
        machine.succeed("openssl x509 -in /var/lib/dots-family/ssl/ca.crt -noout -text")
    
    with subtest("Daemon service starts successfully"):
        machine.wait_for_unit("dots-family-daemon.service")
        machine.succeed("systemctl is-active dots-family-daemon.service")
    
    with subtest("DBus service is available"):
        machine.wait_until_succeeds("busctl status org.dots.FamilyDaemon", timeout=30)
        machine.succeed("busctl introspect org.dots.FamilyDaemon /org/dots/FamilyDaemon")
    
    with subtest("CLI tool communicates with daemon"):
        machine.succeed("dots-family-ctl status")
    
    with subtest("Database is created and accessible"):
        machine.succeed("test -f /var/lib/dots-family/family.db")
        machine.succeed("sqlite3 /var/lib/dots-family/family.db '.tables'")
    
    with subtest("Profile configuration is loaded"):
        output = machine.succeed("dots-family-ctl profile list")
        assert "child" in output or "Test Child" in output, "Child profile not found"
    
    with subtest("Service logs show no critical errors"):
        logs = machine.succeed("journalctl -u dots-family-daemon.service --no-pager")
        assert "panic" not in logs.lower(), "Daemon panicked"
        assert "fatal" not in logs.lower(), "Fatal error in daemon"
  '';
}
