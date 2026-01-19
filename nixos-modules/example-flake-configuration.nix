# Example NixOS configuration for DOTS Family Mode
# This file shows the correct way to integrate the DOTS Family Mode NixOS module
# NO OVERLAYS REQUIRED - uses standard NixOS package building

{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    dots-family-mode.url = "github:your-repo/dots-family-mode";
  };
  
  outputs = { self, nixpkgs, dots-family-mode }: {
    nixosConfigurations.example-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        # Your main configuration
        ./configuration.nix
        
        # Import DOTS Family Mode module
        dots-family-mode.nixosModules.default
      ];
      
      # NO OVERLAY NEEDED - packages are built automatically
    };
  };
}

# Then in your configuration.nix:
{ config, pkgs, ... }: {
  services.dots-family = {
    enable = true;
    parentUsers = [ "parent" ];
    childUsers = [ "child" ];
    reportingOnly = true;  # Start with monitoring only
    
    profiles.child = {
      name = "Child Account";
      ageGroup = "8-12";
      dailyScreenTimeLimit = "2h";
      timeWindows = [{
        start = "15:00";
        end = "18:00";
        days = [ "mon" "tue" "wed" "thu" "fri" ];
      }];
      allowedApplications = [ "firefox" "calculator" "tuxpaint" ];
      webFilteringLevel = "moderate";
    };
  };

  # Don't forget to create the users!
  users.users = {
    parent = {
      isNormalUser = true;
      hashedPassword = "$6$..."; # Use mkpasswd
      extraGroups = [ "wheel" ];
    };
    child = {
      isNormalUser = true;
      hashedPassword = "$6$..."; # Use mkpasswd
    };
  };
}