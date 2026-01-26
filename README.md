# DOTS Family Mode

DOTS Family Mode is a comprehensive parental control and child safety system designed for Linux desktop environments. Built natively in Rust, it provides robust content filtering, application controls, time management, and activity monitoring while maintaining privacy through local-only operation.

## Current Status: Phase 4 Complete - Production Ready

### eBPF Monitoring System ✅
- **5 Production eBPF Monitors** (27.4KB total)
  - `process-monitor` (4.8K): Process exec/exit with PPID and executable paths
  - `filesystem-monitor` (6.8K): File open/read/write/close with full paths
  - `network-monitor` (5.5K): TCP connect/send/recv with bandwidth tracking
  - `memory-monitor` (5.7K): Memory allocations (kmalloc/kfree, page alloc/free)
  - `disk-io-monitor` (4.6K): Block I/O with nanosecond latency tracking
- **16 Probe Functions**: Tracepoints and kprobes for comprehensive monitoring
- **Full Userspace Integration**: eBPF → Monitor → Event Processor → SQLite Database
- **All Tests Passing**: 222 unit tests (100% pass rate)

### Core System
- **Daemon**: Fully functional with complete eBPF monitoring integration
- **Monitor**: Activity tracking service with database storage
- **CLI**: Complete administration tool
- **NixOS Integration**: Declarative module system
- **VM Testing**: Automated test framework available

## Licensing

DOTS Family Mode is **dual-licensed**:

- **AGPLv3** (Open Source): For open source and network use
  - Contact: licensing@dots-family-mode.org
  - Required for modifications and redistribution
  
- **Commercial License**: For commercial closed-source deployments
  - Contact: shift@someone.section.me
  - Case-by-case licensing for proprietary use

See [LICENSE](LICENSE) for complete details.

## Installation

### Method 1: NixOS Module (Recommended)

This method provides full system integration with automatic service management.

```bash
# Add to your configuration.nix
{
  imports = [ ./modules/nixos/dots-family-mode.nix ];
  
  services.dots-family-mode.enable = true;
  
  # Optional: Configure settings
  services.dots-family-mode.settings = {
    logLevel = "info";
    dataDir = "/var/lib/dots-family-mode";
  };
}

# Rebuild and start
sudo nixos-rebuild switch
sudo systemctl enable --now dots-family-mode
```

### Method 2: Development Installation

For developers or testing on non-NixOS systems.

```bash
# Clone repository
git clone https://github.com/dots-distro/dots-family-mode.git
cd dots-family-mode
nix develop

# Build and run
cargo build --workspace --release
sudo ./target/release/dots-family-daemon

# Use CLI
./target/release/dots-family-cli --help
```

## Quick Start

1. **Clone**: `git clone https://github.com/dots-distro/dots-family-mode.git`
2. **Enter Dev Environment**: `nix develop`
3. **Build**: `cargo build --workspace --release`
4. **Configure**: Create first profile and time windows
5. **Start**: Run daemon and begin monitoring

## Getting Help

- **Documentation**: See [docs/](docs/) directory
- **Issues**: [GitHub Issues](https://github.com/dots-distro/dots-family-mode/issues)
- **Discussions**: [GitHub Discussions](https://github.com/dots-distro/dots-family-mode/discussions)
- **Contributing**: See [CONTRIBUTING.md](CONTRIBUTING.md)
- **Security**: See [SECURITY.md](SECURITY.md)

## Community

DOTS Family Mode is now **publicly available** and ready for community adoption! 

### 🎯 Mission
Provide families with enterprise-grade parental controls built on privacy-first principles and advanced eBPF monitoring capabilities.

### 🤝 Contribute
We welcome contributions! Whether you're interested in:
- eBPF programming and kernel monitoring
- Rust development and system programming
- Security research and privacy protection
- User experience and interface design
- Documentation and testing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines and [GitHub Issues](https://github.com/dots-distro/dots-family-mode/issues) to get started.

---

**Repository**: https://github.com/dots-distro/dots-family-mode  
**License**: AGPLv3 (Open Source) / Commercial (Proprietary)  
**Status**: 🏁 Production Ready - Complete eBPF Integration