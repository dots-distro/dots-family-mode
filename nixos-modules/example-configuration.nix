# Example NixOS configuration using DOTS Family Mode
# This file demonstrates how to integrate DOTS Family Mode into a NixOS system

{ config, pkgs, ... }:

{
  imports = [
    # Add DOTS Family Mode module from flake
    # When using as a flake input:
    # inputs.dots-family-mode.nixosModules.default
  ];

  # Enable DOTS Family Mode
  services.dots-family = {
    enable = true;
    
    # Define parent and child users
    parentUsers = [ "mom" "dad" ];
    childUsers = [ "alice" "bob" ];
    
    # Enable features
    enableWebFiltering = true;
    enableTerminalFiltering = false;  # Optional - can be intrusive
    enableNotifications = true;
    
    # Start in reporting mode for initial deployment
    reportingOnly = false;  # Set to true for testing phase
    
    # User profiles with restrictions
    profiles = {
      alice = {
        name = "Alice";
        ageGroup = "8-12";
        dailyScreenTimeLimit = "2h30m";
        
        # Allowed times for computer use
        timeWindows = [
          {
            start = "09:00";
            end = "12:00";
            days = [ "sat" "sun" ];  # Weekend mornings
          }
          {
            start = "15:00";
            end = "17:30";
            days = [ "mon" "tue" "wed" "thu" "fri" ];  # After school
          }
        ];
        
        # Application restrictions
        allowedApplications = [
          "firefox"
          "inkscape"
          "tuxmath"
          "scratch"
          "calculator"
        ];
        
        blockedApplications = [
          "discord"
          "steam"
          "gimp"  # Too complex for age group
        ];
        
        webFilteringLevel = "strict";
      };
      
      bob = {
        name = "Bob";
        ageGroup = "13-17";
        dailyScreenTimeLimit = "4h";
        
        timeWindows = [
          {
            start = "07:00";
            end = "22:00";
            days = [ "sat" "sun" ];  # More freedom on weekends
          }
          {
            start = "15:30";
            end = "21:00";
            days = [ "mon" "tue" "wed" "thu" "fri" ];  # After homework
          }
        ];
        
        allowedApplications = [
          "firefox"
          "discord"  # Allowed for older child
          "vscode"
          "blender"
          "gimp"
        ];
        
        blockedApplications = [
          "steam"  # Gaming restrictions
        ];
        
        webFilteringLevel = "moderate";  # Less restrictive for teens
      };
    };
  };

  # Create the actual user accounts
  users.users = {
    mom = {
      isNormalUser = true;
      description = "Mom";
      extraGroups = [ "wheel" "networkmanager" ];
      hashedPassword = "$6$...";  # Use proper password hash
    };
    
    dad = {
      isNormalUser = true;
      description = "Dad";
      extraGroups = [ "wheel" "networkmanager" ];
      hashedPassword = "$6$...";  # Use proper password hash
    };
    
    alice = {
      isNormalUser = true;
      description = "Alice";
      hashedPassword = "$6$...";  # Use proper password hash
    };
    
    bob = {
      isNormalUser = true;
      description = "Bob";
      hashedPassword = "$6$...";  # Use proper password hash
    };
  };

  # Optional: Desktop environment configuration
  services.xserver = {
    enable = true;
    displayManager.gdm.enable = true;
    desktopManager.gnome.enable = true;
  };

  # Optional: Enable sound and networking
  sound.enable = true;
  hardware.pulseaudio.enable = true;
  networking.networkmanager.enable = true;

  # Install useful applications for children
  environment.systemPackages = with pkgs; [
    # Educational software
    tuxmath
    gcompris
    scratch
    
    # Creative tools (age-appropriate)
    inkscape
    audacity
    
    # Basic utilities
    firefox
    libreoffice
    calculator
  ];

  # System hardening (optional but recommended)
  security.sudo.wheelNeedsPassword = true;
  security.polkit.enable = true;

  system.stateVersion = "23.11"; # Update to match your NixOS version
}