# DOTS Framework Family Mode - Documentation Index

Complete planning documentation for implementing parental controls and child safety features in DOTS Framework.

## Quick Navigation

### Getting Started
- **[README.md](README.md)** - Start here for an overview of Family Mode, its philosophy, and key features

### Architecture & Design
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System architecture, component design, data flow, and communication protocols
- **[DATA_SCHEMA.md](DATA_SCHEMA.md)** - Database schema, encryption, migrations, and data management

### Feature Specifications
- **[PARENTAL_CONTROLS.md](PARENTAL_CONTROLS.md)** - Time limits, application restrictions, age-based profiles, and exceptions
- **[CONTENT_FILTERING.md](CONTENT_FILTERING.md)** - Web filtering, category-based blocking, safe search, and filter lists
- **[MONITORING.md](MONITORING.md)** - Activity tracking, reporting, dashboard features, and privacy controls

### Integration Guides
- **[WM_INTEGRATION.md](WM_INTEGRATION.md)** - Window manager integration for Niri, Swayfx, and Hyprland
- **[TERMINAL_INTEGRATION.md](TERMINAL_INTEGRATION.md)** - Terminal and shell integration, command filtering, and Ghostty support

### Implementation
- **[RUST_APPLICATIONS.md](RUST_APPLICATIONS.md)** - Detailed specifications for all 7 Rust applications
- **[IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md)** - Phased implementation plan with timeline and milestones

## Documentation Statistics

- **Total Documents**: 10 markdown files
- **Total Lines**: 7,963 lines
- **Total Size**: ~210 KB
- **Coverage**: Complete system specification

## Reading Paths

### For Project Managers
1. README.md - Understand the vision
2. IMPLEMENTATION_ROADMAP.md - See timeline and resource requirements
3. PARENTAL_CONTROLS.md - Understand key features
4. MONITORING.md - Understand reporting capabilities

### For Architects
1. README.md - Understand requirements
2. ARCHITECTURE.md - See system design
3. DATA_SCHEMA.md - Review data model
4. WM_INTEGRATION.md - Understand platform integration

### For Developers
1. ARCHITECTURE.md - Understand component interaction
2. RUST_APPLICATIONS.md - See implementation details
3. DATA_SCHEMA.md - Understand data structures
4. Specific integration docs (WM_INTEGRATION.md, TERMINAL_INTEGRATION.md)
5. IMPLEMENTATION_ROADMAP.md - See development phases

### For Testers
1. README.md - Understand features
2. PARENTAL_CONTROLS.md - Test scenarios for controls
3. CONTENT_FILTERING.md - Test scenarios for filtering
4. MONITORING.md - Verify reporting accuracy
5. IMPLEMENTATION_ROADMAP.md - See testing phases

### For End Users (Future)
1. README.md - Understand what Family Mode does
2. PARENTAL_CONTROLS.md - Learn about available controls
3. Installation guide (to be created from IMPLEMENTATION_ROADMAP.md)
4. Configuration examples (in README.md)

## Key Concepts

### Core Components
- **dots-family-daemon** - Central coordination service
- **dots-family-monitor** - Activity tracking service
- **dots-family-filter** - Web content filtering proxy
- **dots-terminal-filter** - Command filtering service
- **dots-wm-bridge** - Window manager integration layer
- **dots-family-ctl** - Command-line administration tool
- **dots-family-gui** - GTK4 graphical dashboard

### Age Groups
- **5-7 years** (Early Elementary) - Heavily restricted, supervised use
- **8-12 years** (Late Elementary/Middle School) - Curated content, filtered web access
- **13-17 years** (High School) - Increasing autonomy with boundaries

### Filter Categories
- **Safety**: Adult, Violence, Gambling, Drugs, Weapons, Hate
- **Activity**: Social Media, Gaming, Video Streaming, Shopping, News
- **Productivity**: Education, Reference, Development, Finance

### Control Types
- **Time Management** - Daily limits, time windows, bedtime mode
- **Application Control** - Allow/block lists, category filtering
- **Web Filtering** - URL/domain filtering, safe search enforcement
- **Terminal Filtering** - Command-level safety with educational feedback

## Implementation Phases

1. **Phase 0** (Weeks 1-2) - Foundation and project setup
2. **Phase 1** (Weeks 3-6) - Core daemon and monitoring
3. **Phase 2** (Weeks 7-10) - Web filtering
4. **Phase 3** (Weeks 11-13) - Multi-WM support
5. **Phase 4** (Weeks 14-17) - Terminal filtering
6. **Phase 5** (Weeks 18-22) - Reporting and dashboard
7. **Phase 6** (Weeks 23-26) - Advanced features
8. **Phase 7** (Weeks 27-30) - NixOS integration
9. **Phase 8** (Weeks 31-34) - Polish and documentation
10. **Phase 9** (Weeks 35-38) - Beta testing
11. **Phase 10** (Week 39+) - Public release

**Total Timeline**: ~9 months for v1.0

## Technical Stack

### Languages & Frameworks
- **Rust** - All system components
- **GTK4** - Graphical interface
- **Nix** - Packaging and deployment

### Key Libraries
- **tokio** - Async runtime
- **zbus** - DBus communication
- **sqlx** - Database access
- **hyper** - HTTP proxy
- **gtk4-rs** - GUI framework
- **argon2** - Password hashing
- **ring** - Cryptography

### Storage
- **SQLite** - Primary database
- **SQLCipher** - Database encryption
- **TOML** - Configuration files

### Protocols
- **DBus** - Inter-process communication
- **Niri IPC** - Niri window manager
- **Sway IPC** - Swayfx window manager
- **Hyprland IPC** - Hyprland window manager

## Contributing to Documentation

When updating documentation:

1. **Maintain consistency** - Follow existing patterns and formatting
2. **Update related docs** - If changing one document, check for related information
3. **Add to this index** - Update INDEX.md with any new sections
4. **Keep examples current** - Ensure code examples remain accurate
5. **Version properly** - Note which version documentation applies to

## Document Relationships

```
README.md
    ├─> ARCHITECTURE.md
    │       ├─> RUST_APPLICATIONS.md
    │       ├─> WM_INTEGRATION.md
    │       ├─> TERMINAL_INTEGRATION.md
    │       └─> DATA_SCHEMA.md
    │
    ├─> PARENTAL_CONTROLS.md
    │       ├─> MONITORING.md
    │       └─> CONTENT_FILTERING.md
    │
    └─> IMPLEMENTATION_ROADMAP.md
            └─> (references all documents)
```

## External Resources

### Related Projects
- [Niri Window Manager](https://github.com/YaLTeR/niri)
- [Swayfx](https://github.com/WillPower3309/swayfx)
- [Hyprland](https://github.com/hyprwm/Hyprland)
- [Ghostty Terminal](https://github.com/ghostty-org/ghostty)

### Relevant Standards
- [XDG Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry-spec/latest/)
- [Wayland Protocol](https://wayland.freedesktop.org/docs/html/)
- [DBus Specification](https://dbus.freedesktop.org/doc/dbus-specification.html)

### Community Resources
- DOTS Framework GitHub: (to be published)
- Discussion Forum: (to be created)
- Issue Tracker: (to be created)

## License

This documentation is part of DOTS Framework and is licensed under the MIT License.

## Changelog

- **2026-01-12**: Initial comprehensive documentation created
  - All 10 documents written
  - Complete system specification
  - Implementation roadmap defined

---

**Last Updated**: January 12, 2026
**Version**: Planning/Pre-Implementation
**Status**: Complete specification, ready for implementation
