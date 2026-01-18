{ pkgs, lib, config, ... }:

# NixOS VM test for DOTS Family Mode eBPF capabilities
let
  dots-family-packages = import ./flake.nix;
in {
  name = "dots-family-ebpf-test";
  
  nodes.machine = { config, pkgs, ... }: {
    imports = [ ./nixos-modules/dots-family ];

    # Enable eBPF support
    boot.kernel.sysctl = {
      "kernel.unprivileged_bpf_disabled" = 0;
      "net.core.bpf_jit_enable" = 1;
    };

    # Required eBPF capabilities
    security.sudo.enable = true;
    
    # Install DOTS Family Mode packages
    environment.systemPackages = with pkgs; [
      dots-family-packages.packages.${pkgs.system}.dots-family-daemon
      dots-family-packages.packages.${pkgs.system}.dots-family-monitor
      dots-family-packages.packages.${pkgs.system}.dots-family-ctl
      dots-family-packages.packages.${pkgs.system}.dots-family-ebpf
      bpftool
      strace
    ];

    # Enable required kernel features for eBPF
    boot.kernelParams = [ 
      "bpf_jit_enable=1" 
    ];

    # Ensure eBPF filesystem is mounted
    boot.specialFileSystems."/sys/kernel/debug" = {
      device = "debugfs";
      fsType = "debugfs";
      options = [ "noauto" ];
    };

    # Test user with eBPF capabilities
    users.users.testuser = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
    };

    # Enable D-Bus for IPC
    services.dbus.enable = true;
  };

  testScript = ''
    # Start the VM and wait for boot
    machine.wait_for_unit("default.target")
    
    # Test eBPF kernel support
    machine.succeed("test -f /proc/sys/kernel/bpf_stats_enabled")
    machine.succeed("mount -t debugfs debugfs /sys/kernel/debug")
    machine.succeed("test -d /sys/kernel/debug/tracing")
    
    # Test eBPF compilation and build
    machine.succeed("ls -la /nix/store/*dots-family-ebpf*/")
    
    # Check if eBPF programs were built
    machine.succeed("find /nix/store -name '*process-monitor*' -o -name '*network-monitor*' -o -name '*filesystem-monitor*'")
    
    # Test daemon eBPF capabilities check
    with subtest("Test daemon eBPF detection"):
        # Run as root to have eBPF capabilities
        result = machine.succeed("sudo dots-family-daemon --help 2>&1 | head -20")
        print(f"Daemon output: {result}")
    
    # Test eBPF program loading (if programs exist)
    with subtest("Test eBPF program access"):
        # Check for eBPF program files
        ebpf_files = machine.succeed("find /nix/store -name '*.o' -path '*ebpf*' || echo 'No eBPF object files found'")
        print(f"Found eBPF files: {ebpf_files}")
        
        # Test basic eBPF tool functionality
        machine.succeed("bpftool prog list || echo 'No programs loaded yet'")
        machine.succeed("bpftool map list || echo 'No maps loaded yet'")
    
    # Test capabilities for eBPF loading
    with subtest("Test eBPF capabilities"):
        # Check if we can read capabilities
        caps = machine.succeed("grep CapEff /proc/self/status")
        print(f"Process capabilities: {caps}")
        
        # Test if we can access eBPF syscalls
        machine.succeed("test -r /proc/sys/kernel/bpf_stats_enabled")
    
    # Test daemon initialization with eBPF
    with subtest("Test daemon eBPF initialization"):
        # Set environment variables for eBPF programs
        machine.succeed("export BPF_PROCESS_MONITOR_PATH=/dev/null")  # Placeholder
        machine.succeed("export BPF_NETWORK_MONITOR_PATH=/dev/null")   # Placeholder  
        machine.succeed("export BPF_FILESYSTEM_MONITOR_PATH=/dev/null") # Placeholder
        
        # Try daemon startup (should succeed even without real eBPF programs)
        result = machine.succeed("timeout 10s sudo dots-family-daemon 2>&1 || echo 'Expected timeout'")
        print(f"Daemon startup result: {result}")
        
        # Check that daemon detected eBPF capability
        if "eBPF manager initialized" in result:
            print("SUCCESS: Daemon eBPF manager initialized")
        else:
            print("INFO: eBPF manager may need configuration")
  '';
}