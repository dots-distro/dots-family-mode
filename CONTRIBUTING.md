# Contributing to DOTS Family Mode

Thank you for your interest in contributing to DOTS Family Mode! This document provides guidelines for contributors.

## License Information

**Dual Licensing**: DOTS Family Mode is dual-licensed:
- **AGPLv3**: For open source and network use (licensing@dots-family-mode.org)
- **Commercial**: For commercial closed-source deployments (shift@someone.section.me)

See [LICENSE](LICENSE) for complete details.

## Getting Started

### Prerequisites
- Nix package manager installed
- Basic familiarity with Rust and Linux system programming
- Understanding of eBPF concepts (for kernel monitoring contributions)

### Development Setup
```bash
# Clone and enter development environment
git clone https://github.com/dots-distro/dots-family-mode.git
cd dots-family-mode
nix develop

# Build all components
cargo build --workspace

# Run tests
cargo test --workspace

# Build test VM
nix build .#nixosConfigurations.dots-family-test-vm.config.system.build.vm
```

## Code Style and Standards

### Rust Code
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow Rust naming conventions
- Add unit tests for new functionality
- Document public APIs with `///` doc comments

### Commit Messages
We follow conventional commits format:
- `feat(component): add new feature`
- `fix(component): fix bug in existing feature`
- `docs(component): update documentation`
- `test(component): add or update tests`
- `refactor(component): refactor code without changing functionality`

Examples:
- `feat(ebpf): add memory allocation monitoring`
- `fix(daemon): handle eBPF load failures gracefully`
- `docs(readme): update installation instructions`
- `test(component): add unit tests for eBPF processors`

### Testing Requirements
- All tests must pass: `cargo test --workspace`
- New features should include unit tests
- eBPF programs must compile in both debug and release
- Integration tests should use VM framework

## Development Areas

### eBPF Monitoring
- Location: `crates/dots-family-ebpf/src/`
- Requirements: Linux kernel knowledge, eBPF restrictions
- Guidelines: Keep programs under 1MB, verify stack usage < 512 bytes

### Userspace Daemon  
- Location: `crates/dots-family-daemon/src/`
- Requirements: Async Rust, database operations
- Guidelines: Use `anyhow::Result` for error handling

### Database Layer
- Location: `crates/dots-family-db/src/`
- Requirements: SQL knowledge, migration management
- Guidelines: Use sqlx for type-safe queries

### NixOS Integration
- Location: `modules/nixos/`
- Requirements: Nix/NixOS experience
- Guidelines: Follow NixOS module patterns

## Pull Request Process

1. **Fork and Branch**
   - Fork the repository
   - Create a feature branch: `git checkout -b feature/your-feature-name`

2. **Develop and Test**
   - Implement your changes
   - Ensure all tests pass
   - Test in VM if applicable

3. **Submit PR**
   - Push to your fork
   - Open a pull request
   - Fill out PR template completely
   - Link any relevant issues

4. **Code Review**
   - Maintainers will review for:
     - Code quality and style
     - Test coverage
     - Documentation updates
     - Breaking changes

5. **Merge**
   - PR must pass CI checks
   - Requires maintainer approval
   - Squash merge for clean history

## Issue Reporting

### Bug Reports
Use the bug report template and include:
- System information (distro, kernel version)
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs

### Feature Requests
Use the feature request template and include:
- Use case description
- Proposed implementation approach
- Priority justification

## Development Guidelines

### eBPF Programs
- Keep under 1MB compiled size
- Monitor stack usage (< 512 bytes)
- Use helper functions for common operations
- Test with multiple kernel versions when possible

### Security Considerations
- All user inputs must be validated
- Follow principle of least privilege
- Document security implications
- Consider timing attacks in parental controls

### Performance
- Database queries should be indexed
- eBPF programs should minimize syscalls
- Monitor CPU and memory usage
- Profile before optimizing

## Community

### Code of Conduct
We are committed to providing a welcoming and inclusive environment. Please see any future CODE_OF_CONDUCT.md for details.

### Getting Help
- GitHub Discussions for questions
- Issues for bug reports and features
- Check documentation first

## Release Process

Releases are managed by maintainers:
1. Update version numbers
2. Update CHANGELOG.md
3. Create git tag
4. Build release artifacts
5. Draft GitHub release with notes

## Licensing Contributions

Please specify in your pull request:
- Which license applies to your contribution (AGPLv3 or Commercial)
- Any commercial licensing requirements

Thank you for contributing to DOTS Family Mode! 🎉