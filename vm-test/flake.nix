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
