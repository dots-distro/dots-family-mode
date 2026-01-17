# DOTS Family Mode

## Overview

DOTS Family Mode is a comprehensive parental control and child safety system designed for Linux desktop environments. Built natively in Rust, it provides robust content filtering, application controls, time management, and activity monitoring while maintaining privacy through local-only operation.

## Current Status: Phase 1 Foundation Complete ✅

The DOTS Family Mode now has a complete foundation infrastructure:

### ✅ Multi-Stage Build System
- **eBPF + SQLx + Nix**: Production-ready build architecture
- **All Components Compile**: daemon, monitor, CLI, eBPF programs
- **Nix Integration**: Full development environment with dependencies

### ✅ Core Infrastructure  
- **eBPF Integration**: Loading and health monitoring for kernel-space programs
- **Database Layer**: SQLx with migrations, encrypted SQLCipher support
- **D-Bus Communication**: Monitor → Daemon activity reporting
- **Policy Engine**: Basic app filtering and enforcement framework

### ✅ Testing & Tooling
- **Comprehensive Tests**: Unit and integration test suites
- **End-to-End Testing**: Full system integration verification
- **Development Tooling**: Formatting, linting, CI/CD ready

### 🚀 Ready For Phase 2
The foundation enables the next development phase:
- **Real eBPF Data Collection**: Actual system monitoring
- **Advanced Policy Features**: Time limits, content filtering
- **GUI Development**: Parent dashboard and child interfaces
- **Production Deployment**: Systemd integration and packaging

## Quick Start

```bash
# Enter development environment
nix develop

# Build all components  
nix build .#default

# Run end-to-end test
./scripts/test_full_system.sh

# Start daemon
./result/bin/dots-family-daemon

# Use CLI
./result/bin/dots-family-ctl status
```

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
- Multi-window manager support (Niri, Sway, Hyprland)
- Ghostty terminal integration
- Shell-level command filtering (bash, zsh, fish)
- DBus-based inter-process communication
- SQLite database for local storage
- Systemd integration for service management

## Core Philosophy

- **Privacy First**: All data stored locally, no cloud dependencies
- **Security**: Tamper-resistant configuration with parental override capabilities
- **Cross-WM Compatible**: Seamless operation across Niri, Swayfx, and Hyprland
- **Age-Appropriate**: Configurable profiles for different developmental stages
- **Performance**: Minimal system overhead through efficient Rust implementation
- **User Experience**: Non-intrusive for children, powerful controls for parents

## Privacy Guarantees

- All data stored locally in SQLite database
- No network communication except for filter list updates (optional)
- No keystroke logging
- No screenshot capture
- Configurable data retention policies
- Parent-only access to monitoring data
- Child notification of monitoring scope

## Security Model

- Configuration files encrypted with parent password
- Kernel-level process monitoring prevents bypass
- Integrity checking of critical system components
- Audit logging of all policy changes
- Secure boot integration (optional)
- Multi-factor authentication for policy changes (optional)

## Development Environment

### Prerequisites
- NixOS or Nix package manager
- direnv (recommended)

### Setup
```bash
# Clone repository
git clone <repository-url>
cd dots-family-mode

# Enter development shell (with direnv)
direnv allow
# OR manually
nix develop

# Build all components
nix build .#default

# Run tests
cargo test --workspace

# Run full system test
./scripts/test_full_system.sh
```

## Documentation

- **docs/ARCHITECTURE.md**: System design and component interaction
- **docs/PARENTAL_CONTROLS.md**: Detailed control mechanisms
- **docs/CONTENT_FILTERING.md**: Filtering system design
- **docs/MONITORING.md**: Monitoring and reporting features
- **docs/RUST_APPLICATIONS.md**: Individual application specifications
- **docs/IMPLEMENTATION_ROADMAP.md**: Phased development plan

## Contributing

Family Mode development requires careful consideration of child safety, privacy, and security. Please review the documentation before contributing.

## License

MIT License (same as DOTS Framework)

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