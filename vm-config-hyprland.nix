# NixOS VM configuration for testing DOTS Family Mode with Hyprland
{ config, pkgs, lib, ... }:

let
  # Import the base configuration
  baseConfig = import ./vm-config.nix { inherit config pkgs lib; };

in lib.recursiveUpdate baseConfig {
  # Override desktop environment to use Hyprland
  services.xserver.enable = false;
  
  # Enable Hyprland
  programs.hyprland = {
    enable = true;
    xwayland.enable = true;
  };
  
  # Install additional Hyprland tools
  environment.systemPackages = baseConfig.environment.systemPackages ++ (with pkgs; [
    hyprland
    hyprpaper
    hypridle
    hyprlock
    wl-clipboard
    mako
    waybar
    rofi-wayland
    foot
    alacritty
    grim
    slurp
  ]);
  
  # Wayland session configuration
  services.greetd = {
    enable = true;
    settings = {
      default_session = {
        command = "${pkgs.greetd.tuigreet}/bin/tuigreet --time --cmd Hyprland";
        user = "greeter";
      };
    };
  };
  
  # Create test user configuration for Hyprland
  system.activationScripts.hyprland-test-setup = ''
    # Create Hyprland config for parent user
    mkdir -p /home/parent/.config/hypr
    cat > /home/parent/.config/hypr/hyprland.conf << EOF
# Hyprland configuration for DOTS Family Mode testing

# Monitor setup
monitor=,preferred,auto,auto

# Start DOTS Family Mode monitor
exec-once = dots-family-monitor
exec-once = waybar

# Environment variables
env = XCURSOR_SIZE,24

# Input configuration
input {
    kb_layout = us
    follow_mouse = 1
    touchpad {
        natural_scroll = no
    }
    sensitivity = 0
}

# General settings
general {
    gaps_in = 5
    gaps_out = 20
    border_size = 2
    col.active_border = rgba(33ccffee) rgba(00ff99ee) 45deg
    col.inactive_border = rgba(595959aa)
    layout = dwindle
}

# Decoration
decoration {
    rounding = 10
    blur {
        enabled = true
        size = 3
        passes = 1
    }
    drop_shadow = yes
    shadow_range = 4
    shadow_render_power = 3
    col.shadow = rgba(1a1a1aee)
}

# Animations
animations {
    enabled = yes
    bezier = myBezier, 0.05, 0.9, 0.1, 1.05
    animation = windows, 1, 7, myBezier
    animation = windowsOut, 1, 7, default, popin 80%
    animation = border, 1, 10, default
    animation = borderangle, 1, 8, default
    animation = fade, 1, 7, default
    animation = workspaces, 1, 6, default
}

# Layout
dwindle {
    pseudotile = yes
    preserve_split = yes
}

# Window rules
windowrule = float, ^(rofi)$

# Keybindings
\$mainMod = SUPER

# Applications
bind = \$mainMod, Return, exec, foot
bind = \$mainMod, F, exec, firefox
bind = \$mainMod, Q, killactive,
bind = \$mainMod, M, exit,
bind = \$mainMod, D, exec, rofi -show drun

# Movement
bind = \$mainMod, left, movefocus, l
bind = \$mainMod, right, movefocus, r
bind = \$mainMod, up, movefocus, u
bind = \$mainMod, down, movefocus, d

# Workspaces
bind = \$mainMod, 1, workspace, 1
bind = \$mainMod, 2, workspace, 2
bind = \$mainMod, 3, workspace, 3
bind = \$mainMod, 4, workspace, 4
bind = \$mainMod, 5, workspace, 5

# Move to workspace
bind = \$mainMod SHIFT, 1, movetoworkspace, 1
bind = \$mainMod SHIFT, 2, movetoworkspace, 2
bind = \$mainMod SHIFT, 3, movetoworkspace, 3
bind = \$mainMod SHIFT, 4, movetoworkspace, 4
bind = \$mainMod SHIFT, 5, movetoworkspace, 5

# Mouse bindings
bindm = \$mainMod, mouse:272, movewindow
bindm = \$mainMod, mouse:273, resizewindow
EOF
    
    # Create similar config for child user
    mkdir -p /home/child/.config/hypr
    cp /home/parent/.config/hypr/hyprland.conf /home/child/.config/hypr/hyprland.conf
    
    # Set ownership
    chown -R parent:users /home/parent/.config/hypr
    chown -R child:users /home/child/.config/hypr
  '';
  
  # Environment variables for Hyprland
  environment.sessionVariables = {
    XDG_CURRENT_DESKTOP = "Hyprland";
    XDG_SESSION_TYPE = "wayland";
    QT_QPA_PLATFORM = "wayland";
    CLUTTER_BACKEND = "wayland";
    SDL_VIDEODRIVER = "wayland";
    WLR_NO_HARDWARE_CURSORS = "1";
    NIXOS_OZONE_WL = "1";
  };
  
  # Enable needed services
  security.polkit.enable = true;
  services.dbus.enable = true;
  xdg.portal = {
    enable = true;
    extraPortals = with pkgs; [ xdg-desktop-portal-hyprland ];
    config.common.default = "*";
  };
}