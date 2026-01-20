# NixOS VM configuration for testing DOTS Family Mode with Sway
{ config, pkgs, lib, ... }:

let
  # Import the base configuration
  baseConfig = import ./vm-config.nix { inherit config pkgs lib; };

in lib.recursiveUpdate baseConfig {
  # Override desktop environment to use Sway
  services.xserver.enable = false;
  
  # Enable Sway
  programs.sway = {
    enable = true;
    wrapperFeatures.gtk = true;
    extraPackages = with pkgs; [
      swaylock
      swayidle
      wl-clipboard
      mako
      alacritty
      foot
      dmenu
      waybar
      firefox
    ];
  };
  
  # Install additional Sway tools
  environment.systemPackages = baseConfig.environment.systemPackages ++ (with pkgs; [
    sway
    swaybg
    swayimg
    grim
    slurp
  ]);
  
  # Wayland session configuration
  services.greetd = {
    enable = true;
    settings = {
      default_session = {
        command = "${pkgs.greetd.tuigreet}/bin/tuigreet --time --cmd sway";
        user = "greeter";
      };
    };
  };
  
  # Create test user configuration for Sway
  system.activationScripts.sway-test-setup = ''
    # Create Sway config for parent user
    mkdir -p /home/parent/.config/sway
    cat > /home/parent/.config/sway/config << EOF
# Default config for sway
set \$mod Mod4
set \$left h
set \$down j
set \$up k
set \$right l
set \$term foot
set \$menu dmenu_path | dmenu | xargs swaymsg exec --

# Start DOTS Family Mode monitor
exec dots-family-monitor

# Start a terminal
bindsym \$mod+Return exec \$term

# Start browser  
bindsym \$mod+f exec firefox

# Kill focused window
bindsym \$mod+Shift+q kill

# Start your launcher
bindsym \$mod+d exec \$menu

# Reload the configuration file
bindsym \$mod+Shift+c reload

# Exit sway
bindsym \$mod+Shift+e exec swaynag -t warning -m 'Do you want to exit sway?' -b 'Yes, exit sway' 'swaymsg exit'

# Moving around
bindsym \$mod+\$left focus left
bindsym \$mod+\$down focus down
bindsym \$mod+\$up focus up
bindsym \$mod+\$right focus right

# Layout stuff
bindsym \$mod+b splith
bindsym \$mod+v splitv

# Switch the current container between different layout styles
bindsym \$mod+s layout stacking
bindsym \$mod+w layout tabbed
bindsym \$mod+e layout toggle split

# Make the current focus fullscreen
bindsym \$mod+Shift+f fullscreen

# Status Bar
bar {
    position top
    status_command while date +'%Y-%m-%d %l:%M:%S %p'; do sleep 1; done
    colors {
        statusline #ffffff
        background #323232
        inactive_workspace #32323200 #32323200 #5c5c5c
    }
}

# Workspaces
bindsym \$mod+1 workspace number 1
bindsym \$mod+2 workspace number 2
bindsym \$mod+3 workspace number 3
bindsym \$mod+4 workspace number 4
bindsym \$mod+5 workspace number 5

# Move focused container to workspace
bindsym \$mod+Shift+1 move container to workspace number 1
bindsym \$mod+Shift+2 move container to workspace number 2
bindsym \$mod+Shift+3 move container to workspace number 3
bindsym \$mod+Shift+4 move container to workspace number 4
bindsym \$mod+Shift+5 move container to workspace number 5
EOF
    
    # Create similar config for child user
    mkdir -p /home/child/.config/sway
    cp /home/parent/.config/sway/config /home/child/.config/sway/config
    
    # Set ownership
    chown -R parent:users /home/parent/.config/sway
    chown -R child:users /home/child/.config/sway
  '';
  
  # Environment variables for Sway
  environment.sessionVariables = {
    XDG_CURRENT_DESKTOP = "sway";
    XDG_SESSION_TYPE = "wayland";
    QT_QPA_PLATFORM = "wayland";
    CLUTTER_BACKEND = "wayland";
    SDL_VIDEODRIVER = "wayland";
    WLR_NO_HARDWARE_CURSORS = "1";
  };
  
  # Enable needed services
  security.polkit.enable = true;
  services.dbus.enable = true;
  xdg.portal = {
    enable = true;
    wlr.enable = true;
  };
}