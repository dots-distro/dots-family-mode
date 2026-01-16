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
        ({ config, pkgs, lib, ... }: {
          # VM settings - use minimal working configuration
          virtualisation = {
            memorySize = 4096;
            graphics = true;
          };
          
          # Don't require a physical disk
          boot.loader.grub.device = lib.mkForce "nodev";
          fileSystems."/".device = lib.mkForce "/dev/disk/by-label/nixos";
        })
      ];
    };
  };
}
