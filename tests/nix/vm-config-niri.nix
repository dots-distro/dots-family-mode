# NixOS VM configuration for testing DOTS Family Mode with Niri
{ config, pkgs, lib, ... }:

let
  # Import the base configuration
  baseConfig = import ./vm-config.nix { inherit config pkgs lib; };
  
  # Niri window manager configuration
  niriConfig = {
    programs.niri = {
      enable = true;
      settings = {
        spawn-at-startup = [
          { command = ["firefox"]; }
        ];
        
        binds = {
          "Mod+T".action = { spawn = ["foot"]; };
          "Mod+Q".action = { close-window = {}; };
          "Mod+M".action = { quit = {}; };
        };
        
        layout = {
          gaps = 8;
          center-focused-column = "never";
          preset-column-widths = [
            { proportion = 0.33333; }
            { proportion = 0.5; }
            { proportion = 0.66667; }
          ];
        };
        
        window-rules = [];
      };
    };
  };

in lib.recursiveUpdate baseConfig {
  # Override desktop environment to use Niri
  services.xserver.enable = false;
  
  # Enable Niri
  imports = [ niriConfig ];
  
  # Install Niri-specific packages
  environment.systemPackages = baseConfig.environment.systemPackages ++ (with pkgs; [
    niri
    foot
    waybar
    fuzzel
  ]);
  
  # Wayland session configuration
  services.greetd = {
    enable = true;
    settings = {
      default_session = {
        command = "${pkgs.greetd.tuigreet}/bin/tuigreet --time --cmd niri";
        user = "greeter";
      };
    };
  };
  
  # Create test user configuration for Niri
  system.activationScripts.niri-test-setup = ''
    # Create Niri config for parent user
    mkdir -p /home/parent/.config/niri
    cat > /home/parent/.config/niri/config.kdl << EOF
spawn-at-startup "dots-family-monitor"
spawn-at-startup "waybar"

binds {
    "Mod+T" { spawn "foot"; }
    "Mod+F" { spawn "firefox"; }
    "Mod+Q" { close-window; }
    "Mod+M" { quit; }
}

layout {
    gaps 8
    center-focused-column "never"
    
    preset-column-widths {
        proportion 0.33333
        proportion 0.5
        proportion 0.66667
    }
}
EOF
    
    # Create similar config for child user
    mkdir -p /home/child/.config/niri
    cp /home/parent/.config/niri/config.kdl /home/child/.config/niri/config.kdl
    
    # Set ownership
    chown -R parent:users /home/parent/.config/niri
    chown -R child:users /home/child/.config/niri
  '';
  
  # Environment variables for Niri
  environment.sessionVariables = {
    XDG_CURRENT_DESKTOP = "niri";
    XDG_SESSION_TYPE = "wayland";
    QT_QPA_PLATFORM = "wayland";
    CLUTTER_BACKEND = "wayland";
    SDL_VIDEODRIVER = "wayland";
  };
}