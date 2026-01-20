#!/usr/bin/env nix-shell
#! nix-shell -i bash -p qemu

# DOTS Family Mode VM Test Script
# This script creates a minimal NixOS VM for testing the DOTS Family Mode system service

set -euo pipefail

VM_NAME="dots-family-test"
VM_DIR="./vm-test"
DISK_SIZE="8G"
MEMORY="4G"

echo "=== Setting up DOTS Family Mode Test VM ==="

# Create VM directory
mkdir -p "$VM_DIR"
cd "$VM_DIR"

# Create a minimal NixOS configuration
cat > configuration.nix << 'EOF'
{ config, pkgs, ... }:

{
  # Boot configuration
  boot.loader.grub.enable = true;
  boot.loader.grub.device = "/dev/vda";

  # Networking
  networking.hostName = "dots-family-test";
  networking.networkmanager.enable = true;
  networking.firewall.enable = false;  # Disable for testing

  # Users
  users.users.root.password = "root";
  users.users.test = {
    isNormalUser = true;
    password = "test";
    extraGroups = [ "wheel" "networkmanager" ];
  };

  # SSH for access
  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "yes";
    settings.PasswordAuthentication = true;
  };

  # Basic packages
  environment.systemPackages = with pkgs; [
    vim
    git
    cargo
    rustc
    sqlite
    pkg-config
    dbus
    systemd
    busctl
    firefox
    foot
    htop
  ];

  # Desktop environment
  services.xserver = {
    enable = true;
    displayManager.lightdm.enable = true;
    windowManager.i3.enable = true;
  };

  # Development tools
  nix.settings.experimental-features = [ "nix-command" "flakes" ];
  
  # System state version
  system.stateVersion = "24.05";
}
EOF

# Create disk image if it doesn't exist
if [ ! -f disk.qcow2 ]; then
  echo "Creating VM disk image..."
  qemu-img create -f qcow2 disk.qcow2 "$DISK_SIZE"
fi

echo ""
echo "=== VM Configuration Created ==="
echo "VM Directory: $(pwd)"
echo "Configuration: configuration.nix"
echo "Disk: disk.qcow2 ($DISK_SIZE)"
echo ""
echo "To build and run the VM:"
echo "1. cd vm-test"
echo "2. nixos-rebuild build-vm --flake .#"
echo "3. ./result/bin/run-*-vm"
echo ""
echo "Or use the quick start script:"
echo "  ./start-vm.sh"

# Create start script
cat > start-vm.sh << 'EOF'
#!/bin/bash
set -e

echo "Building NixOS VM..."
nix build --no-link .#nixosConfigurations.dots-family-test.config.system.build.vm

echo "Starting VM..."
./result/bin/run-dots-family-test-vm -m 4096 -smp 4
EOF

chmod +x start-vm.sh

# Create flake for the VM
cat > flake.nix << 'EOF'
{
  description = "DOTS Family Mode Test VM";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
  };

  outputs = { self, nixpkgs }: {
    nixosConfigurations.dots-family-test = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        
        # VM-specific configuration
        ({ config, pkgs, ... }: {
          # VM settings
          virtualisation = {
            memorySize = 4096;
            cores = 4;
            graphics = true;
            diskSize = 8192;
          };
          
          # Don't require a physical disk
          boot.loader.grub.device = "nodev";
          fileSystems."/".device = "/dev/disk/by-label/nixos";
          
          # Enable nested virtualization for development
          virtualisation.vmVariant = {
            virtualisation = {
              memorySize = 4096;
              cores = 4;
            };
          };
        })
      ];
    };
  };
}
EOF

echo "VM setup complete!"
echo ""
echo "Next steps:"
echo "1. cd vm-test"
echo "2. Copy DOTS Family Mode source into VM"
echo "3. Build and test as system service"