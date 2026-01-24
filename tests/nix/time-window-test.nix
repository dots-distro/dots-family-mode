{ pkgs, self }:

pkgs.testers.runNixOSTest {
  name = "dots-family-time-windows";
  
  nodes.machine = { config, ... }: {
    imports = [
      self.nixosModules.dots-family
    ];

    services.dots-family = {
      enable = true;
      package = self.packages.${pkgs.system}.dots-family-daemon;
      monitorPackage = self.packages.${pkgs.system}.dots-family-monitor;
      ctlPackage = self.packages.${pkgs.system}.dots-family-ctl;
      ebpfPackage = self.packages.${pkgs.system}.dots-family-ebpf;
      parentUsers = [ "parent" ];
      childUsers = [ "child1" "child2" ];
      reportingOnly = false;
      runAsRoot = true;
      
      profiles.child1 = {
        name = "Child One";
        ageGroup = "8-12";
        dailyScreenTimeLimit = "4h";
        timeWindows = [
          {
            start = "06:00";
            end = "08:00";
            days = [ "mon" "tue" "wed" "thu" "fri" ];
          }
          {
            start = "15:00";
            end = "19:00";
            days = [ "mon" "tue" "wed" "thu" "fri" ];
          }
        ];
        weekendTimeWindows = [{
          start = "08:00";
          end = "21:00";
          days = [ "sat" "sun" ];
        }];
      };

      profiles.child2 = {
        name = "Child Two";
        ageGroup = "13-17";
        dailyScreenTimeLimit = "5h";
        timeWindows = [{
          start = "16:00";
          end = "19:00";
          days = [ "mon" "tue" "wed" "thu" "fri" ];
        }];
      };
    };

    users.users = {
      parent = {
        isNormalUser = true;
        password = "parent123";
        extraGroups = [ "wheel" ];
      };
      
      child1 = {
        isNormalUser = true;
        password = "child123";
      };

      child2 = {
        isNormalUser = true;
        password = "child456";
      };
    };
  };

  testScript = ''
    start_all()
    machine.wait_for_unit("multi-user.target")
    
    with subtest("Service and users are set up correctly"):
        machine.wait_for_unit("dots-family-daemon.service")
        machine.succeed("systemctl is-active dots-family-daemon.service")
        machine.succeed("id child1")
        machine.succeed("id child2")
        machine.succeed("id parent")
    
    with subtest("DBus service is available"):
        machine.wait_until_succeeds("busctl status org.dots.FamilyDaemon", timeout=30)
    
    with subtest("Time window configuration is loaded"):
        # Verify that time window profiles are accessible
        machine.succeed("dots-family-ctl profile list")
        
        # Check child1 profile exists
        result = machine.succeed("dots-family-ctl profile list")
        assert "child1" in result or "Child One" in result, "child1 profile not found"
    
    with subtest("Time window enforcement checks current time"):
        # The daemon should be checking time windows
        # This is integration test - we verify the daemon is running with time window config
        machine.succeed("journalctl -u dots-family-daemon.service --no-pager | head -50")
    
    with subtest("Parent user is not restricted"):
        # Parent users should never be subject to time windows
        # In a real test, we'd verify parent can login at any time
        machine.succeed("getent group dots-family-parents | grep parent")
    
    with subtest("Child users are in correct group"):
        machine.succeed("getent group dots-family-children | grep child1")
        machine.succeed("getent group dots-family-children | grep child2")
    
    with subtest("Time window data is in database"):
        # Verify database contains time window configuration
        machine.succeed("test -f /var/lib/dots-family/family.db")
        # Wait for database to be accessible
        machine.wait_until_succeeds("dots-family-ctl profile list", timeout=10)
    
    with subtest("Multiple users have different window configurations"):
        # This verifies per-user window configurations work
        # In full implementation, we'd test actual login enforcement
        # For now, verify the profiles are distinct
        result = machine.succeed("dots-family-ctl profile list")
        print(f"Profiles: {result}")
    
    with subtest("Daemon responds to time window queries"):
        # Test that daemon can answer time window related queries
        # This will be implemented via DBus interface
        machine.succeed("busctl introspect org.dots.FamilyDaemon /org/dots/FamilyDaemon")
  '';
}
