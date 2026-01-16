# DOTS Framework Family Mode

## Overview

DOTS Framework Family Mode is a comprehensive parental control and child safety system designed for Linux desktop environments. Built natively in Rust, it provides robust content filtering, application controls, time management, and activity monitoring while maintaining privacy through local-only operation.

## Core Philosophy

- **Privacy First**: All data stored locally, no cloud dependencies
- **Security**: Tamper-resistant configuration with parental override capabilities
- **Cross-WM Compatible**: Seamless operation across Niri, Swayfx, and Hyprland
- **Age-Appropriate**: Configurable profiles for different developmental stages
- **Performance**: Minimal system overhead through efficient Rust implementation
- **User Experience**: Non-intrusive for children, powerful controls for parents

## Key Features

### Parental Controls
- Time-based access restrictions (daily limits, time windows)
- Application allow/block lists with category-based filtering
- Age-appropriate profile templates (5-7, 8-12, 13-17)
- Progressive access expansion as children mature
- Emergency override and temporary exceptions

### Content Filtering
- Web content filtering with category-based blocking
- Terminal command filtering without breaking legitimate workflows
- Application content inspection for known threat patterns
- Real-time threat detection and blocking
- Custom filter rule creation

### Monitoring & Reporting
- Activity logging with configurable retention
- Daily/weekly usage reports
- Alert system for policy violations
- Screen time analytics
- Application usage statistics

### Technical Features
- Multi-window manager support (Niri, Swayfx, Hyprland)
- Ghostty terminal integration
- Shell-level command filtering (bash, zsh, fish)
- DBus-based inter-process communication
- SQLite database for local storage
- Systemd integration for service management

## Architecture Overview

Family Mode consists of eight Rust applications working together:

1. **dots-family-daemon**: Core service managing policies and enforcement
2. **dots-family-monitor**: Application and window monitoring
3. **dots-family-filter**: Content filtering engine
4. **dots-family-ctl**: CLI administration tool
5. **dots-family-gui**: GTK4-based parent dashboard
6. **dots-terminal-filter**: Terminal command filter plugin
7. **dots-wm-bridge**: Window manager integration layer
8. **dots-family-lockscreen**: Custom lockscreen with parental override

## Age-Based Profiles

### Ages 5-7 (Early Elementary)
- Heavily restricted application access
- No web browser access without supervision
- Pre-approved applications only
- 1-2 hours screen time limit
- Simple activity logging

### Ages 8-12 (Late Elementary/Middle School)
- Curated application categories (educational, creative, games)
- Filtered web access with safe search enforcement
- 2-3 hours screen time limit
- Terminal access blocked
- Detailed activity monitoring

### Ages 13-17 (High School)
- Category-based restrictions with exceptions
- Web filtering with mature content blocking
- 3-4 hours screen time limit
- Limited terminal access with command filtering
- Privacy-respecting monitoring

## Security Model

- Configuration files encrypted with parent password
- Kernel-level process monitoring prevents bypass
- Integrity checking of critical system components
- Audit logging of all policy changes
- Secure boot integration (optional)
- Multi-factor authentication for policy changes (optional)

## Privacy Guarantees

- All data stored locally in SQLite database
- No network communication except for filter list updates (optional)
- No keystroke logging
- No screenshot capture
- Configurable data retention policies
- Parent-only access to monitoring data
- Child notification of monitoring scope

## Integration Points

### NixOS/Home Manager
- Declarative configuration via Nix modules
- Profile-based deployment
- Automatic service setup
- Integration with existing DOTS Framework modules

### Window Managers
- Niri: Native Rust integration via IPC
- Swayfx: Wayland protocol integration
- Hyprland: Socket-based control

### Terminal Emulators
- Ghostty: Native plugin architecture
- Other terminals: PTY-level filtering fallback

### Desktop Environment
- XDG Desktop Portal integration
- System tray notifications
- Parent authentication dialogs

## Documentation Structure

- **ARCHITECTURE.md**: System design and component interaction
- **PARENTAL_CONTROLS.md**: Detailed control mechanisms
- **CONTENT_FILTERING.md**: Filtering system design
- **MONITORING.md**: Monitoring and reporting features
- **RUST_APPLICATIONS.md**: Individual application specifications
- **WM_INTEGRATION.md**: Window manager integration details
- **TERMINAL_INTEGRATION.md**: Terminal and shell integration
- **DATA_SCHEMA.md**: Database schema and data structures
- **IMPLEMENTATION_ROADMAP.md**: Phased development plan

## Quick Start (Future)

```nix
# In Home Manager configuration
features.family-mode = {
  enable = true;
  role = "parent"; # or "child"

  profiles.child = {
    name = "Alex";
    ageGroup = "8-12";
    screenTime.dailyLimit = "2h";
    applications.mode = "allowlist";
    applications.allowed = [
      "firefox" # with filtering
      "inkscape"
      "tuxmath"
    ];
  };
};
```

## Development Status

This is planning documentation. Implementation follows the roadmap in IMPLEMENTATION_ROADMAP.md.

## Contributing

Family Mode development requires careful consideration of child safety, privacy, and security. See CONTRIBUTING.md (to be created) for guidelines.

## License

Same as DOTS Framework (MIT).

## Ethical Considerations

Family Mode is designed to:
- Support parental guidance, not replace it
- Respect children's growing autonomy
- Maintain transparency about monitoring
- Protect privacy while ensuring safety
- Provide age-appropriate boundaries

Parents should:
- Communicate openly about rules and monitoring
- Adjust restrictions as children mature
- Use monitoring data to start conversations, not punishment
- Respect children's reasonable privacy expectations
- Model healthy technology use

## Support

- Issues: GitHub issue tracker
- Discussions: GitHub discussions
- Documentation: docs/improvements/family_mode/
- Examples: examples/family-mode/

## Related Projects

- Qustodio (commercial, proprietary)
- Net Nanny (commercial, proprietary)
- OpenDNS FamilyShield (DNS-based only)
- KDE Parental Controls (desktop-specific)

Family Mode provides a FOSS alternative with superior Linux integration and privacy guarantees.
