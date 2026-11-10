# DOTS Framework Family Mode - Documentation Index

Complete documentation for DOTS Family Mode parental control system.

## Quick Navigation

### Getting Started
- **[README.md](README.md)** - Project overview and quick start commands
- **[history/README.md](history/README.md)** - Historical session notes and status reports (not current instructions)

### Project Policies
- **[../LICENSING.md](../LICENSING.md)** - AGPL-3.0-or-later and commercial licensing
- **[../SECURITY.md](../SECURITY.md)** - Supported versions, private reporting, and scope
- **[history/README.md](history/README.md)** - Historical session notes, kept for traceability only

### Architecture & Design
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System architecture, component design, data flow, and communication protocols
- **[DATA_SCHEMA.md](DATA_SCHEMA.md)** - Database schema, encryption, migrations, and data management
- **[SECURITY_ARCHITECTURE.md](SECURITY_ARCHITECTURE.md)** - Security model, hardening, and protection mechanisms

### Feature Specifications
- **[PARENTAL_CONTROLS.md](PARENTAL_CONTROLS.md)** - Time limits, application restrictions, age-based profiles, and exceptions
- **[CONTENT_FILTERING.md](CONTENT_FILTERING.md)** - Web filtering, category-based blocking, safe search, and filter lists
- **[MONITORING.md](MONITORING.md)** - Activity tracking, reporting, dashboard features, and privacy controls

### Integration Guides
- **[WM_INTEGRATION.md](WM_INTEGRATION.md)** - Window manager integration for Niri, Sway, and Hyprland
- **[TERMINAL_INTEGRATION.md](TERMINAL_INTEGRATION.md)** - Terminal and shell integration, command filtering
- **[NIXOS_INTEGRATION.md](NIXOS_INTEGRATION.md)** - NixOS module configuration and deployment

### Implementation
- **[RUST_APPLICATIONS.md](RUST_APPLICATIONS.md)** - Detailed specifications for all Rust applications
- **[EBPF_ENHANCEMENTS.md](EBPF_ENHANCEMENTS.md)** - eBPF monitoring implementation (Phase 1-3 complete)
- **[EBPF_QUICK_REFERENCE.md](EBPF_QUICK_REFERENCE.md)** - Quick reference for all 5 eBPF monitors
- **[PHASE3_INTEGRATION_PLAN.md](PHASE3_INTEGRATION_PLAN.md)** - Phase 3 eBPF integration testing plan

## Current Implementation Status

See [../README.md](../README.md#status) for the authoritative status. CI is
currently red, so the feature list below describes code present in the tree,
not verified behaviour.

### Components present in the tree (Phase 8 layout)

1. **Core Daemon** - Policy enforcement with eBPF monitoring
2. **Activity Monitor** - Window tracking and session monitoring
3. **CLI Administration** - Complete dots-family-ctl tool
4. **NixOS Integration** - Declarative module system
5. **Systemd Services** - Service management and startup
6. **DBus Communication** - Inter-process communication
7. **Web Filtering** - Content filter proxy
8. **Terminal Filtering** - Command safety checking
9. **VM Testing Framework** - Automated testing
10. **Security Hardening** - Capability restrictions

### 🚧 In Progress

- Phase 3 eBPF integration testing (monitors ready, userspace integration pending)
- GUI dashboard (dots-family-gui - disabled due to compilation)
- Enhanced reporting features

### 📋 Planned

- Advanced analytics and reporting
- Mobile companion app
- Cloud sync (optional)

## Documentation Statistics

- **Total Documents**: 15+ markdown files
- **Coverage**: Complete system specification and implementation

## Documentation Sections

### Core Documentation
| Document | Status | Last Updated | Description |
|----------|--------|--------------|-------------|
| README.md | ✅ Current | 2026-01-21 | Quick start and commands |
| ARCHITECTURE.md | ✅ Current | 2026-01-12 | System design |
| DATA_SCHEMA.md | ✅ Current | 2026-01-12 | Database design |
| SECURITY_ARCHITECTURE.md | ✅ Current | 2026-01-12 | Security model |

### Feature Documentation
| Document | Status | Last Updated | Description |
|----------|--------|--------------|-------------|
| PARENTAL_CONTROLS.md | ✅ Current | 2026-01-12 | Control mechanisms |
| CONTENT_FILTERING.md | ✅ Current | 2026-01-12 | Web filtering |
| MONITORING.md | ✅ Current | 2026-01-12 | Activity monitoring |
| WM_INTEGRATION.md | ✅ Current | 2026-01-12 | Window manager support |
| TERMINAL_INTEGRATION.md | ✅ Current | 2026-01-12 | Terminal filtering |

### Integration Documentation
| Document | Status | Last Updated | Description |
|----------|--------|--------------|-------------|
| NIXOS_INTEGRATION.md | ✅ Current | 2026-01-21 | NixOS modules |
| RUST_APPLICATIONS.md | ✅ Current | 2026-01-12 | Rust applications |
| history/README.md | ✅ Current | 2026-02-07 | Historical session notes |
| EBPF_ENHANCEMENTS.md | ✅ Current | 2026-01-26 | eBPF Phase 1-3 |
| EBPF_QUICK_REFERENCE.md | ✅ Current | 2026-01-26 | eBPF quick reference |
| PHASE3_INTEGRATION_PLAN.md | ✅ Current | 2026-01-26 | Integration testing |

### Project Policies
| Document | Status | Description |
|----------|--------|-------------|
| ../LICENSING.md | ✅ Current | Licence summary and commercial contact |
| ../SECURITY.md | ✅ Current | Vulnerability reporting and security scope |

### Test Evidence
| Document | Status | Description |
|----------|--------|-------------|
| history/README.md | ✅ Current | Historical test and session records, not current results |

## Reading Paths

### For Project Managers
1. README.md - Understand the project status
2. ARCHITECTURE.md - See completed features
3. NIXOS_INTEGRATION.md - Deployment options

### For Architects
1. README.md - Current implementation status
2. ARCHITECTURE.md - System design
3. SECURITY_ARCHITECTURE.md - Security model

### For Developers
1. README.md - Build and test commands
2. NIXOS_INTEGRATION.md - Module configuration
3. ARCHITECTURE.md - Component interaction
4. EBPF_QUICK_REFERENCE.md - eBPF monitor quick reference
5. EBPF_ENHANCEMENTS.md - eBPF monitoring implementation
6. PHASE3_INTEGRATION_PLAN.md - eBPF integration testing
7. Source code in crates/

### For Testers
1. README.md - Quick start
2. history/README.md - Historical test and session records (not current results)
3. Source tests in tests/ and per-crate `#[cfg(test)]` modules

## Key Concepts

### Core Components (present in the tree)
- ✅ **dots-family-daemon** - Central coordination service
- ✅ **dots-family-monitor** - Activity tracking service
- ✅ **dots-family-filter** - Web content filtering proxy
- ✅ **dots-terminal-filter** - Command filtering service
- ✅ **dots-family-ctl** - Command-line administration tool
- 🚧 **dots-family-gui** - GTK4 dashboard (disabled)
- ✅ **dots-wm-bridge** - Window manager integration

### Age Groups
- **5-7 years** (Early Elementary) - Heavily restricted
- **8-12 years** (Late Elementary/Middle School) - Curated content
- **13-17 years** (High School) - Increasing autonomy

### Control Types
- **Time Management** - Daily limits, time windows
- **Application Control** - Allow/block lists
- **Web Filtering** - URL/domain filtering
- **Terminal Filtering** - Command-level safety

## Technical Stack

### Languages & Frameworks
- **Rust** - All system components
- **Nix** - Packaging and deployment
- **SQLite/SQLCipher** - Database with encryption

### Key Libraries
- **tokio** - Async runtime
- **zbus** - DBus communication
- **sqlx** - Database access
- **hyper** - HTTP proxy
- **aya** - eBPF programs

### Protocols
- **DBus** - Inter-process communication
- **Wayland IPC** - Window manager integration

## Contributing

When updating documentation:

1. **Maintain consistency** - Follow existing patterns
2. **Update related docs** - Check for related information
3. **Add to this index** - Update INDEX.md with new sections
4. **Keep examples current** - Ensure code examples are accurate
5. **Note version** - Document which version features apply to

## External Resources

### Related Projects
- [Niri Window Manager](https://github.com/YaLTeR/niri)
- [Sway](https://github.com/swaywm/sway)
- [Hyprland](https://github.com/hyprwm/Hyprland)

### Relevant Standards
- [XDG Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry-spec/latest/)
- [Wayland Protocol](https://wayland.freedesktop.org/docs/html/)
- [DBus Specification](https://dbus.freedesktop.org/doc/dbus-specification.html)

## Changelog

- **2026-01-26**: Phase 3 eBPF monitoring complete
  - Added 5 eBPF monitors (16 probes total, 27.4KB)
  - memory-monitor: kernel allocations tracking
  - disk-io-monitor: block I/O latency monitoring
  - network-monitor: TCP bandwidth tracking (enhanced)
  - Created PHASE3_INTEGRATION_PLAN.md
  - Updated DEVELOPMENT.md with advanced patterns

- **2026-01-21**: Updated for Phase 8 completion
  - README.md comprehensive updates
  - VM testing guide added
  - All build commands validated
  - NixOS integration documented
  
- **2026-01-12**: Initial comprehensive documentation
  - All 10 documents written
  - Complete system specification
  - Implementation roadmap defined

---

**Last Updated**: February 7, 2026
**Status**: Early beta; CI is red. See [../README.md](../README.md#status).
**Version**: 0.1.0
