# DOTS Family Mode - Development Guide

This guide provides detailed information for developers who want to contribute to or modify DOTS Family Mode.

## Table of Contents

1. [Development Environment Setup](#development-environment-setup)
2. [Project Architecture](#project-architecture)
3. [Building the Project](#building-the-project)
4. [Testing Strategy](#testing-strategy)
5. [eBPF Development](#ebpf-development)
6. [Code Organization](#code-organization)
7. [Development Workflow](#development-workflow)
8. [Debugging Tips](#debugging-tips)
9. [Contributing Guidelines](#contributing-guidelines)

---

## Development Environment Setup

### Prerequisites

- **Nix Package Manager**: Required for reproducible builds
  ```bash
  curl -L https://nixos.org/nix/install | sh
  ```

- **direnv** (optional but recommended): Automatic environment loading
  ```bash
  nix-env -i direnv
  echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
  ```

- **rustup** (for eBPF development): Required for nightly Rust
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup toolchain install nightly
  ```

### Setting Up the Development Environment

1. **Clone the repository**:
   ```bash
   git clone <repository-url>
   cd dots-family-mode
   ```

2. **Enable direnv** (if installed):
   ```bash
   echo "use flake" > .envrc
   direnv allow
   ```

3. **Enter the Nix development shell**:
   ```bash
   nix develop
   ```

   This provides:
   - Rust stable toolchain for userspace code
   - Rust nightly toolchain for eBPF programs
   - Development tools (rust-analyzer, cargo-watch, etc.)
   - GTK4 and GUI development libraries
   - Testing frameworks and utilities

4. **Verify the setup**:
   ```bash
   cargo --version
   cargo test --workspace --lib --bins
   ```

### Editor Configuration

#### VS Code

Install extensions:
- `rust-analyzer`: Rust language support
- `direnv`: Automatic environment loading
- `NixIDE`: Nix language support

Configuration (`.vscode/settings.json`):
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.procMacro.enable": true
}
```

#### Neovim/Vim

Use `rust-tools.nvim` or `coc-rust-analyzer` with direnv support.

---

## Project Architecture

### Component Overview

```
dots-family-mode/
├── crates/
│   ├── dots-family-daemon/      # Core policy enforcement (root)
│   ├── dots-family-monitor/     # User session monitoring
│   ├── dots-family-ctl/         # CLI administration tool
│   ├── dots-family-gui/         # GTK4 graphical interface
│   ├── dots-terminal-filter/    # Terminal command filtering
│   ├── dots-family-common/      # Shared types and utilities
│   ├── dots-family-ebpf/        # eBPF monitoring programs
│   └── dots-wm-bridge/          # Window manager integration
├── nixos-modules/               # NixOS integration modules
├── prebuilt-ebpf/              # Pre-compiled eBPF binaries
├── tests/                      # Integration and smoke tests
└── docs/                       # Documentation
```

### Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Kernel Space                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  eBPF Programs (process/network/filesystem monitors)  │  │
│  │  - Process execution tracking                         │  │
│  │  - Network connection monitoring                      │  │
│  │  - File system access tracking                        │  │
│  └───────────────────────────────────────────────────────┘  │
│                         ↓ Ring Buffers                       │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│                    User Space (Root)                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  dots-family-daemon                                   │  │
│  │  - Load & manage eBPF programs                        │  │
│  │  - Process events from ring buffers                   │  │
│  │  - Enforce policies & restrictions                    │  │
│  │  - Store activity logs in SQLite                      │  │
│  └───────────────────────────────────────────────────────┘  │
│                    ↕ DBus (system bus)                       │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│                    User Space (User)                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  dots-family-monitor                                  │  │
│  │  - Monitor user session activity                      │  │
│  │  - Track window focus & app usage                     │  │
│  │  - Report to daemon via DBus                          │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  dots-family-gui / dots-family-ctl                    │  │
│  │  - User interfaces for configuration                  │  │
│  │  - Activity reports & parental controls              │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Language**: Rust (stable for userspace, nightly for eBPF)
- **eBPF**: aya-rs framework for kernel monitoring
- **GUI**: GTK4 with Rust bindings
- **IPC**: DBus for inter-process communication
- **Database**: SQLite for activity logs
- **Build System**: Nix flakes for reproducible builds
- **Deployment**: NixOS modules for declarative configuration

---

## Building the Project

### Full Build

Build all components:
```bash
nix build .#default
```

This produces:
- `./result/bin/dots-family-daemon`
- `./result/bin/dots-family-monitor`
- `./result/bin/dots-family-ctl`
- `./result/bin/dots-family-gui`
- `./result/bin/dots-terminal-filter`

### Development Build

Quick incremental builds during development:
```bash
# Enter devShell
nix develop

# Build all crates
cargo build --workspace --bins

# Build specific crate
cargo build -p dots-family-daemon

# Build with optimizations
cargo build --release -p dots-family-daemon
```

### Component-Specific Builds

```bash
# Daemon only
nix build .#dots-family-daemon

# Monitor only
nix build .#dots-family-monitor

# CLI tool
nix build .#dots-family-ctl

# GUI
nix build .#dots-family-gui

# Terminal filter
nix build .#dots-terminal-filter
```

### Building Documentation

```bash
# Generate API documentation
cargo doc --workspace --no-deps --open

# Build user guide (if using mdbook)
mdbook build docs/
```

---

## Testing Strategy

See [TESTING.md](./TESTING.md) for comprehensive testing documentation.

### Quick Test Commands

```bash
# Unit tests (fast, ~90 seconds)
cargo test --workspace --lib --bins

# Smoke test (very fast, <5 seconds)
./tests/smoke-test.sh

# Integration tests
cargo test --workspace --test '*'

# Run specific test
cargo test -p dots-family-daemon test_policy_enforcement
```

### Test Organization

- **Unit tests**: `crates/*/src/**/*.rs` (inline `#[cfg(test)]` modules)
- **Integration tests**: `crates/*/tests/**/*.rs`
- **Smoke tests**: `tests/smoke-test.sh`
- **VM tests**: `tests/vm-test-minimal.nix` (use sparingly, very slow)

---

## eBPF Development

### Overview

eBPF programs are pre-compiled and stored in `prebuilt-ebpf/` to avoid requiring nightly Rust on production systems.

### Building eBPF Programs

eBPF programs **must** be built with rustup nightly, not Nix Rust:

```bash
cd crates/dots-family-ebpf

# Set up nightly toolchain PATH
export PATH="$HOME/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin:$PATH"

# Build eBPF programs
cargo build --release --target bpfel-unknown-none -Z build-std=core

# Update prebuilt binaries
cp target/bpfel-unknown-none/release/*-monitor ../../prebuilt-ebpf/

# Verify eBPF format
file ../../prebuilt-ebpf/process-monitor
# Output: ELF 64-bit LSB relocatable, eBPF, version 1 (SYSV), not stripped
```

### eBPF Program Structure

Each eBPF program follows this pattern:

```rust
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::*,
    macros::{map, kprobe},
    maps::RingBuf,
    programs::ProbeContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Event {
    pub pid: u32,
    pub data: [u8; 256],
}

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(1024 * 1024, 0);

#[kprobe]
pub fn trace_function(ctx: ProbeContext) -> u32 {
    let event = Event {
        pid: (unsafe { bpf_get_current_pid_tgid() } >> 32) as u32,
        data: [0; 256],
    };
    
    if let Some(mut buf) = EVENTS.reserve::<Event>(0) {
        buf.write(event);
        buf.submit(0);
    }
    
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
```

### eBPF Limitations

1. **Stack Size**: 512 bytes maximum
   - Use per-CPU arrays for large buffers
   - Example: `PerCpuArray<[u8; 512]>`

2. **No Standard Library**: `#![no_std]` environment
   - No heap allocations
   - No panic unwinding
   - Limited string operations

3. **Verifier Constraints**:
   - All loops must be bounded
   - No unbounded recursion
   - Pointer arithmetic must be safe

4. **Helper Functions**: Only kernel-approved helpers available
   - See aya-rs documentation for available helpers
   - Common: `bpf_get_current_pid_tgid`, `bpf_probe_read_kernel`

### eBPF Enhancement Phases

See [EBPF_ENHANCEMENTS.md](./EBPF_ENHANCEMENTS.md) for detailed roadmap.

**Phase 1 (✅ Complete):**
- PID, UID, GID extraction
- Process name (comm) extraction
- Basic network and filesystem monitoring

**Phase 2 (✅ Complete):**
- PPID extraction (parent process tracking)
- Executable path extraction (__data_loc pointer parsing)
- Socket address/port extraction (struct sock access)
- Filename extraction (user space memory reads)

**Phase 3 (✅ Complete):**
- Memory monitoring (kmalloc/kfree, page alloc/free)
- Disk I/O with latency tracking (HashMap-based)
- Enhanced network metrics (tcp_sendmsg/recvmsg bandwidth)
- 5 production monitors, 16 probe functions, 27.4KB total

### Phase 3 Advanced eBPF Patterns

**Pattern 1: HashMap for Stateful Tracking**
```rust
#[map]
static PENDING_IO: HashMap<u64, u64> = HashMap::with_max_entries(10240, 0);

// Store start time on I/O issue
unsafe { PENDING_IO.insert(&request_id, &timestamp, 0); }

// Calculate latency on I/O complete
let start = unsafe { PENDING_IO.get(&request_id).copied().unwrap_or(0) };
let latency = current_time.saturating_sub(start);

// Cleanup
unsafe { PENDING_IO.remove(&request_id); }
```

**Pattern 2: Dynamic Size Calculations**
```rust
// Read page order from tracepoint
let order: u32 = unsafe { ctx.read_at(16).unwrap_or(0) };

// Calculate actual size: (2^order) * PAGE_SIZE
let size = (1u64 << order) * 4096;
```

**Pattern 3: Multiple Probes Per Monitor**
```rust
// network-monitor.rs has 3 kprobes:
#[kprobe] pub fn tcp_connect()   // Connection establishment
#[kprobe] pub fn tcp_sendmsg()   // TX bandwidth tracking
#[kprobe] pub fn tcp_recvmsg()   // RX bandwidth tracking
```

**Pattern 4: Bandwidth Tracking**
```rust
// Extract bytes from function arguments
let bytes_sent = ctx.arg::<u64>(2).unwrap_or(0);

let event = NetworkEvent {
    bytes_sent,
    bytes_received: 0,
    // ... other fields
};
```

### Debugging eBPF Programs

1. **Use `bpftool` to inspect loaded programs**:
   ```bash
   sudo bpftool prog list
   sudo bpftool map list
   ```

2. **Check kernel logs for verifier errors**:
   ```bash
   sudo dmesg | grep -i bpf
   ```

3. **Use `bpf_printk` for debugging** (development only):
   ```rust
   use aya_ebpf::helpers::bpf_trace_printk;
   
   let msg = b"Debug value: %d\n\0";
   unsafe { bpf_trace_printk(msg.as_ptr(), msg.len() as u32, pid) };
   ```
   
   View output:
   ```bash
   sudo cat /sys/kernel/debug/tracing/trace_pipe
   ```

---

## Code Organization

### Crate Structure

#### `dots-family-common`
Shared types and utilities used across all components.

**Key modules**:
- `monitoring.rs`: eBPF event types
- `policy.rs`: Policy and restriction definitions
- `config.rs`: Configuration types
- `database.rs`: SQLite schema and queries

#### `dots-family-daemon`
Core policy enforcement daemon (runs as root).

**Key modules**:
- `main.rs`: Daemon initialization and main loop
- `monitoring_service.rs`: eBPF program loading and event processing
- `policy_engine.rs`: Policy evaluation and enforcement
- `dbus_service.rs`: DBus interface implementation
- `activity_logger.rs`: Activity logging to SQLite

#### `dots-family-monitor`
User session monitoring service.

**Key modules**:
- `main.rs`: Monitor initialization
- `session_tracker.rs`: User session activity tracking
- `window_monitor.rs`: Window focus and app usage tracking
- `dbus_client.rs`: DBus communication with daemon

#### `dots-family-ctl`
Command-line administration tool.

**Key modules**:
- `main.rs`: CLI argument parsing
- `commands/`: Subcommand implementations
  - `profile.rs`: User profile management
  - `policy.rs`: Policy configuration
  - `activity.rs`: Activity reports
  - `status.rs`: System status

#### `dots-family-gui`
GTK4 graphical interface.

**Key modules**:
- `main.rs`: GTK application initialization
- `window.rs`: Main window implementation
- `views/`: Different view implementations
  - `dashboard.rs`: Overview dashboard
  - `profiles.rs`: Profile management view
  - `activity.rs`: Activity reports view
  - `settings.rs`: Settings view

#### `dots-terminal-filter`
Terminal command filtering and educational system.

**Key modules**:
- `main.rs`: Shell wrapper entry point
- `command_parser.rs`: Command parsing and analysis
- `risk_analyzer.rs`: Command risk assessment
- `educational.rs`: Educational messages for users
- `config.rs`: Filter configuration

#### `dots-family-ebpf`
eBPF monitoring programs (kernel space).

**Files**:
- `src/process-monitor.rs`: Process execution monitoring
- `src/network-monitor.rs`: Network connection monitoring
- `src/filesystem-monitor.rs`: File system access monitoring

### Naming Conventions

- **Modules**: `snake_case` (e.g., `policy_engine.rs`)
- **Structs/Enums**: `PascalCase` (e.g., `PolicyRule`, `ActivityEvent`)
- **Functions**: `snake_case` (e.g., `load_configuration`, `apply_policy`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_BUFFER_SIZE`)
- **Crates**: `kebab-case` (e.g., `dots-family-daemon`)

### Error Handling

Use `anyhow` for application errors and `thiserror` for library errors:

```rust
// Application code (daemon, CLI, GUI)
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let config = fs::read_to_string("config.toml")
        .context("Failed to read config file")?;
    toml::from_str(&config)
        .context("Failed to parse config")
}

// Library code (common, wm-bridge)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Invalid policy rule: {0}")]
    InvalidRule(String),
    
    #[error("Policy not found: {0}")]
    NotFound(String),
}
```

---

## Development Workflow

### Feature Development

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/new-feature
   ```

2. **Make changes and test frequently**:
   ```bash
   # Quick feedback loop
   cargo check -p dots-family-daemon
   cargo test -p dots-family-daemon
   ```

3. **Run full test suite before committing**:
   ```bash
   cargo test --workspace --lib --bins
   cargo clippy --workspace -- -D warnings
   cargo fmt --check
   ```

4. **Commit with descriptive messages**:
   ```bash
   git add <files>
   git commit -m "feat(daemon): add new feature

   - Detailed description
   - Technical changes
   - Breaking changes (if any)"
   ```

### Commit Message Format

Follow conventional commits:

```
type(scope): short description

Longer description of the change, including:
- Why the change was made
- Technical details
- Breaking changes
- Related issues

Examples:
- feat(daemon): add process monitoring
- fix(cli): correct argument parsing
- docs(readme): update installation guide
- refactor(common): simplify policy types
- test(daemon): add policy engine tests
```

### Code Review Checklist

Before submitting for review:

- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation updated (if needed)
- [ ] Commit messages are clear
- [ ] No debug code or commented-out sections
- [ ] Error handling is appropriate
- [ ] Tests added for new functionality

### Continuous Integration

The CI pipeline runs:

1. **Build**: All components build successfully
2. **Test**: Unit and integration tests pass
3. **Lint**: Clippy checks pass
4. **Format**: Code is properly formatted
5. **eBPF**: eBPF programs compile without errors

---

## Debugging Tips

### Debugging the Daemon

1. **Run daemon in foreground with logging**:
   ```bash
   RUST_LOG=debug sudo ./result/bin/dots-family-daemon
   ```

2. **Check systemd logs**:
   ```bash
   sudo journalctl -u dots-family-daemon.service -f
   ```

3. **Inspect eBPF programs**:
   ```bash
   sudo bpftool prog show
   sudo bpftool map dump name PROCESS_EVENTS
   ```

4. **Debug with GDB**:
   ```bash
   sudo gdb --args ./result/bin/dots-family-daemon
   (gdb) run
   (gdb) bt
   ```

### Debugging eBPF Issues

1. **Check verifier output**:
   ```bash
   sudo bpftool prog load process-monitor obj process-monitor.o type tracepoint
   ```

2. **Monitor kernel tracing**:
   ```bash
   sudo cat /sys/kernel/debug/tracing/trace_pipe
   ```

3. **Check for stack issues**:
   - Stack limit is 512 bytes
   - Use per-CPU arrays for large buffers
   - Check struct sizes: `std::mem::size_of::<Event>()`

### Debugging GUI Issues

1. **Run with GTK debugging**:
   ```bash
   GTK_DEBUG=interactive ./result/bin/dots-family-gui
   ```

2. **Check DBus communication**:
   ```bash
   dbus-monitor --system
   ```

3. **Use `glade` for UI inspection**:
   - Load `.ui` files in Glade
   - Inspect widget hierarchy

### Common Issues

**Issue**: eBPF program fails to load
- **Solution**: Check kernel version (5.15+ required)
- **Solution**: Verify CAP_BPF and CAP_NET_ADMIN capabilities
- **Solution**: Check dmesg for verifier errors

**Issue**: Tests fail with permission denied
- **Solution**: Some tests require root (eBPF tests)
- **Solution**: Run with `sudo -E cargo test`

**Issue**: Build fails with "cannot find crate"
- **Solution**: Ensure you're in `nix develop` shell
- **Solution**: Run `cargo clean && cargo build`

**Issue**: GUI doesn't connect to daemon
- **Solution**: Verify daemon is running: `systemctl status dots-family-daemon`
- **Solution**: Check DBus permissions
- **Solution**: Review daemon logs for errors

---

## Contributing Guidelines

### Getting Started

1. **Fork the repository** on your Git hosting platform

2. **Clone your fork**:
   ```bash
   git clone <your-fork-url>
   cd dots-family-mode
   ```

3. **Add upstream remote**:
   ```bash
   git remote add upstream <original-repo-url>
   ```

4. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature
   ```

### Making Changes

1. **Keep commits focused**: One logical change per commit

2. **Write clear commit messages**: Follow conventional commits format

3. **Add tests**: Cover new functionality with unit/integration tests

4. **Update documentation**: Keep docs in sync with code changes

5. **Run quality checks**:
   ```bash
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   cargo fmt --check
   ```

### Submitting Changes

1. **Rebase on latest upstream**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Push to your fork**:
   ```bash
   git push origin feature/your-feature
   ```

3. **Create a Pull Request**:
   - Provide clear description
   - Reference related issues
   - List breaking changes (if any)

### Code Style

- Follow Rust style guidelines
- Use `cargo fmt` for formatting
- Fix `cargo clippy` warnings
- Prefer explicit over implicit
- Comment complex logic

### Testing Requirements

- Unit tests for new functions
- Integration tests for features
- No reduction in test coverage
- All existing tests must pass

### Documentation Requirements

- Update README for user-facing changes
- Add doc comments for public APIs
- Update architecture docs for significant changes
- Include examples in doc comments

---

## Additional Resources

- [Rust Programming Language](https://doc.rust-lang.org/book/)
- [aya eBPF Book](https://aya-rs.dev/book/)
- [GTK4 Documentation](https://gtk-rs.org/)
- [Nix Package Manager](https://nixos.org/manual/nix/stable/)
- [DBus Specification](https://dbus.freedesktop.org/doc/dbus-specification.html)

---

## Getting Help

- **Issues**: File bugs and feature requests in the issue tracker
- **Discussions**: Ask questions in GitHub Discussions
- **Chat**: Join the developer chat (if available)
- **Email**: Contact maintainers directly

---

## License

See [LICENSE](../LICENSE) file for license information.
