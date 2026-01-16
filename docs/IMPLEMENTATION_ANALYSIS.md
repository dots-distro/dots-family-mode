# Family Mode Implementation Analysis

## Executive Summary

After comprehensive review of the Family Mode architecture and requirements, this document provides:
1. **Feasibility Assessment**: Technical and timeline analysis
2. **Phase 0 Breakdown**: Detailed task breakdown for foundation work
3. **Critical Design Decisions**: Key architectural choices needed upfront
4. **Risk Analysis**: Potential blockers and mitigation strategies

## Current State

**dots-detection**: Feature-complete system information collector
- 60 tests (all passing)
- CI/CD configured
- Documentation complete
- Ready for integration as data source for Family Mode

**Family Mode**: Design phase only, no implementation yet

## Architecture Assessment

### Complexity Analysis

| Component | Complexity | Estimated Time | Critical Dependencies |
|-----------|------------|----------------|----------------------|
| dots-family-common | Low | 1-2 days | None |
| dots-family-daemon | High | 2-3 weeks | Database, DBus, Policy engine |
| dots-family-monitor | Medium | 1-2 weeks | WM bridge, DBus |
| dots-family-filter | High | 2-3 weeks | HTTP proxy, Pattern matching |
| dots-family-ctl | Low | 3-5 days | Daemon DBus interface |
| dots-family-gui | Medium | 2-3 weeks | GTK4, Daemon interface |
| dots-terminal-filter | Medium | 1-2 weeks | Shell integration |
| dots-wm-bridge | Medium | 1-2 weeks | WM-specific protocols |

**Total Estimated Time**: 12-18 weeks (3-4.5 months) for MVP

### Technical Challenges

1. **DBus Inter-Process Communication**
   - Requires zbus expertise
   - Complex async coordination
   - Error handling across process boundaries
   - **Mitigation**: Start with simple RPC patterns, iterate

2. **Window Manager Integration**
   - Three WMs with different protocols (Niri, Swayfx, Hyprland)
   - Wayland protocol complexity
   - **Mitigation**: Focus on Niri first (native Rust), defer others

3. **Content Filtering**
   - HTTP/HTTPS proxy implementation
   - Certificate handling for HTTPS inspection
   - Performance requirements
   - **Mitigation**: Use existing proxy libraries, simple pattern matching first

4. **Database Encryption**
   - SQLCipher integration
   - Key management and derivation
   - Migration between schema versions
   - **Mitigation**: Use sqlx with SQLCipher feature

5. **Shell Command Filtering**
   - Multiple shell support (bash, zsh, fish)
   - PTY integration without breaking workflows
   - Performance impact on terminal
   - **Mitigation**: Start with bash only, use PROMPT_COMMAND hook

## Phase 0 Deep Dive

### Goal
Establish foundation for all applications without requiring cross-component integration.

### Timeline
**2 weeks** (80 hours at 20 hrs/week)

### Tasks Breakdown

#### Week 1: Workspace and Common Libraries (40 hours)

**Day 1-2: Workspace Setup (16 hours)**
- Create Cargo workspace structure
- Configure workspace-level dependencies
- Set up clippy, rustfmt, deny.toml
- Configure GitHub Actions CI
- Create development environment (devShell)
- Document build process

**Day 3-4: dots-family-common (16 hours)**
- Define error types hierarchy
- Create configuration types (profiles, policies, rules)
- Implement DBus interface traits
- Add logging utilities
- Write unit tests
- Document public API

**Day 5: dots-family-proto (8 hours)**
- Define DBus interface specifications
- Create event type enums
- Implement message serialization
- Write schema validation tests

#### Week 2: Database and Dev Tools (40 hours)

**Day 1-2: Database Foundation (16 hours)**
- Set up SQLCipher with sqlx
- Create migration system (using sqlx-cli)
- Implement core schema (profiles, policies, activity_logs)
- Write database access layer
- Add connection pooling
- Test migration rollback

**Day 3: Development Tools (8 hours)**
- Create mock daemon (returns canned responses)
- Create mock WM adapter (simulates windows)
- Write test data generators
- Document local dev setup

**Day 4-5: Integration and Documentation (16 hours)**
- Verify all crates build together
- Set up integration test harness
- Write architecture documentation
- Create contribution guide
- Document testing strategy

### Deliverables Checklist

- [ ] Cargo workspace with 7 crate directories
- [ ] dots-family-common with error types, config types, DBus traits
- [ ] dots-family-proto with interface definitions
- [ ] SQLite database with migrations and access layer
- [ ] Mock daemon and WM adapter for testing
- [ ] CI pipeline running on all crates
- [ ] Development documentation
- [ ] All tests passing

### Critical Design Decisions Required

#### 1. DBus Service Names
**Decision Needed**: System bus or session bus?

**Options**:
- **Session bus** (recommended): One daemon per user, simpler permissions
  - Service: `org.dots.FamilyDaemon.<username>`
  - Object: `/org/dots/FamilyDaemon`
- **System bus**: One daemon for all users, more complex
  - Service: `org.dots.FamilyDaemon`
  - Requires polkit integration

**Recommendation**: Session bus for MVP, system bus for v2.

#### 2. Database Encryption Key Management
**Decision Needed**: How to derive and store encryption keys?

**Options**:
- **Password-derived** (recommended for Phase 0):
  - Parent password → Argon2 → DB key
  - Pros: No additional storage, user-controlled
  - Cons: Password change requires re-encryption
- **Hardware-backed** (future):
  - TPM or secure enclave
  - Pros: Better security, no password management
  - Cons: Requires TPM, more complex

**Recommendation**: Password-derived for Phase 0.

#### 3. Configuration Format
**Decision Needed**: TOML vs JSON vs RON?

**Options**:
- **TOML** (recommended): Human-readable, standard in Rust
- **JSON**: Universal, verbose
- **RON**: Rust-native, less known

**Recommendation**: TOML for user-facing configs, JSON for database storage.

#### 4. Policy Enforcement Strategy
**Decision Needed**: Proactive blocking vs reactive monitoring?

**Options**:
- **Proactive** (recommended): Block before app launches
  - Requires process monitoring (eBPF or procfs polling)
  - Better UX (instant feedback)
- **Reactive**: Detect after launch, kill process
  - Simpler implementation
  - Worse UX (app flickers)

**Recommendation**: Proactive for Phase 1, requires procfs polling initially.

#### 5. WM Integration Approach
**Decision Needed**: Protocol choice for each WM?

| WM | Protocol | Complexity |
|----|----------|------------|
| Niri | Native IPC (Unix socket) | Low - Rust |
| Swayfx | Sway IPC protocol | Medium - JSON over socket |
| Hyprland | Hyprland socket protocol | Medium - Custom format |

**Recommendation**: Implement Niri adapter in Phase 1, defer others to Phase 2.

### Risk Analysis

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| DBus async complexity | High | High | Start simple, use zbus examples |
| Performance overhead | Medium | High | Profile early, optimize hot paths |
| WM protocol changes | Medium | Medium | Version detection, fallback modes |
| Scope creep | High | High | Strict Phase 0 scope, defer features |
| SQLCipher build issues | Low | Medium | Use vendored SQLCipher |
| Shell integration breaks workflows | Medium | High | Extensive testing, escape hatch |

### Success Criteria for Phase 0

1. **Buildability**: `cargo build` succeeds on all crates
2. **Testing**: `cargo test` passes with >70% coverage on common libs
3. **CI**: GitHub Actions runs tests on every commit
4. **Documentation**: README in each crate, API docs on public items
5. **Database**: Can create, migrate, and query database
6. **Mocks**: Can simulate daemon and WM for testing

### Non-Goals for Phase 0

- No actual policy enforcement
- No real WM integration
- No content filtering
- No GUI
- No NixOS packaging

These are deferred to Phase 1+.

## Phase 1 Preview

After Phase 0 completion, Phase 1 focuses on:

1. **dots-family-daemon MVP**:
   - Load policies from database
   - Respond to status queries
   - Enforce time limits (simple check)
   - No authentication yet

2. **dots-family-monitor MVP**:
   - Poll Niri windows every 1 second
   - Track active application
   - Report to daemon via DBus

3. **dots-wm-bridge (Niri only)**:
   - Implement WindowManagerAdapter trait
   - Parse Niri IPC responses
   - Provide window list

4. **dots-family-ctl MVP**:
   - `status` command (show active profile, remaining time)
   - `policy list` command
   - `activity log` command (last 24 hours)

**Timeline**: 4 weeks after Phase 0

## Recommended Immediate Next Steps

1. **Create workspace structure** (1 hour)
2. **Implement dots-family-common core types** (4 hours)
3. **Set up database with migrations** (4 hours)
4. **Create CI pipeline** (2 hours)
5. **Write integration test harness** (3 hours)

**Total**: ~14 hours for critical path to "buildable workspace with tests"

## eBPF vs Procfs Decision

For process monitoring, we have two options:

### Procfs Polling (Recommended for Phase 0-1)
**Pros**:
- Simple, no root required
- Works everywhere
- Easy to debug
- Current dots-detection already has this

**Cons**:
- 1-2% CPU overhead
- Can miss short-lived processes
- No kernel-level enforcement

### eBPF (Defer to Phase 2+)
**Pros**:
- <0.1% overhead
- Catches all processes
- Kernel-level enforcement
- Modern approach

**Cons**:
- Requires root or CAP_BPF
- Complex BPF program development
- Verifier constraints
- Kernel version dependency

**Recommendation**: Use procfs polling for MVP, migrate to eBPF in Phase 2 once core features work.

## Conclusion

Family Mode is a **substantial undertaking** but architecturally sound. The phased approach is realistic:

- **Phase 0** (2 weeks): Foundation - ACHIEVABLE NOW
- **Phase 1** (4 weeks): MVP daemon + monitor - Next milestone
- **Phase 2+** (6+ weeks): Full feature set

**Recommendation**: Proceed with Phase 0 implementation immediately. Focus on quality over speed - this is security-critical software for children.

## Open Questions for User

1. **Target NixOS version**: Which NixOS release should we target?
2. **Rust toolchain**: Stable, or can we use nightly for async traits?
3. **WM priority**: Confirm Niri is highest priority?
4. **Deployment**: NixOS module only, or support other distros?
5. **Testing**: How to test safely? (Don't want to lock out real users)
