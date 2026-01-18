# DOTS Family Mode - Session Continuation Guide

## Project Overview

**Location:** `/home/shift/code/endpoint-agent/dots-family-mode/`

**Project:** DOTS Family Mode - A comprehensive parental control and child safety system for Linux, implemented in Rust. This is a production-ready family safety framework built as a multi-crate Rust workspace.

**Related Project:** `../dots-detection/` contains comprehensive design documentation in the `docs/` directory.

## Current Status: Phase 0 MOSTLY COMPLETE (85%)

### What We've Implemented

Successfully implemented **foundation infrastructure** with 39 Rust source files:

#### 1. **dots-family-common** (Shared Types & Config) - COMPLETE
**Location:** `crates/dots-family-common/`

**Implemented Files:**
- `src/lib.rs` - Module exports
- `src/error.rs` - Error types hierarchy
- `src/types.rs` - Core types (Profile, Policy, Rule, AgeGroup, TimeWindow, etc.)
- `src/config.rs` - Configuration types

**Key Features:**
- Comprehensive error types with proper Error trait implementation
- Profile types with age-based defaults (5-7, 8-12, 13-17)
- Policy enforcement types (time windows, app filtering, screen time)
- Serde serialization for all types

#### 2. **dots-family-proto** (DBus Protocol) - COMPLETE
**Location:** `crates/dots-family-proto/`

**Implemented Files:**
- `src/lib.rs` - Module exports
- `src/daemon.rs` - Daemon DBus interface definitions
- `src/monitor.rs` - Monitor DBus interface definitions
- `src/events.rs` - Event types for inter-process communication

**Key Features:**
- DBus interface traits using zbus 4.0
- Event type enums (ActivityEvent, PolicyEvent, SessionEvent)
- Serialization support for complex types

#### 3. **dots-family-db** (Database Layer) - COMPLETE
**Location:** `crates/dots-family-db/`

**Implemented Files:**
- `src/lib.rs` - Module exports and basic tests
- `src/error.rs` - Database-specific errors
- `src/connection.rs` - Database connection and pooling
- `src/migrations.rs` - Migration system integration
- `src/models.rs` - Database model types
- `src/queries/mod.rs` - Query module organization
- `src/queries/profiles.rs` - Profile CRUD operations
- `src/queries/sessions.rs` - Session management queries
- `src/queries/activities.rs` - Activity logging queries
- `src/queries/events.rs` - Event logging queries

**Key Features:**
- SQLCipher integration via sqlx (encryption at rest)
- Connection pooling
- Migration system ready (migrations in `migrations/` directory)
- Comprehensive query layer for all database operations

#### 4. **dots-family-daemon** (Core Service) - 80% COMPLETE
**Location:** `crates/dots-family-daemon/`

**Implemented Files:**
- `src/main.rs` - Tokio async runtime with structured logging
- `src/daemon.rs` - Main service orchestration
- `src/config.rs` - Configuration management
- `src/dbus_impl.rs` - DBus interface implementation
- `src/profile_manager.rs` - Profile management
- `src/session_manager.rs` - Session lifecycle tracking
- `src/policy_engine.rs` - Policy enforcement logic
- `tests/integration_test.rs` - Integration tests (3 FAILING, 4 passing)

**Status:** Core functionality implemented, but needs database integration fixes

**Known Issues:**
- 3 integration tests failing (needs daemon running or mock improvements)
- Database integration incomplete

#### 5. **dots-family-monitor** (Activity Tracking) - 90% COMPLETE
**Location:** `crates/dots-family-monitor/`

**Implemented Files:**
- `src/main.rs` - Tokio async runtime
- `src/config.rs` - Monitoring configuration
- `src/monitor.rs` - Main monitoring loop
- `src/wayland.rs` - Multi-compositor window tracking

**Key Features:**
- Auto-detects compositor (Niri/Sway/Hyprland)
- Polls focused window every 1 second
- Extracts app_id, window title, PID

**Missing:** Integration with daemon to report activity

#### 6. **dots-family-ctl** (CLI Tool) - 80% COMPLETE
**Location:** `crates/dots-family-ctl/`

**Implemented Files:**
- `src/main.rs` - Clap-based CLI parser
- `src/commands/mod.rs` - Command module structure
- `src/commands/profile.rs` - Profile management commands
- `src/commands/status.rs` - System status display
- `src/commands/check.rs` - Application permission checking

**Commands Implemented:**
```bash
dots-family-ctl profile list
dots-family-ctl profile show <name>
dots-family-ctl profile create <name> <age-group>
dots-family-ctl status
dots-family-ctl check <app-id>
```

**Status:** CLI structure complete, needs daemon integration testing

#### 7. **Placeholder Crates** (Not Implemented)
These have placeholder `main.rs` or `lib.rs` only:
- `dots-family-filter` - Content filtering (placeholder)
- `dots-family-gui` - GTK4 parent dashboard (placeholder)
- `dots-terminal-filter` - Terminal command filtering (placeholder)
- `dots-wm-bridge` - Window manager integration (placeholder)

### Infrastructure Complete

**Build System:**
- ✅ Cargo workspace configured (`Cargo.toml`)
- ✅ All 10 crates defined as workspace members
- ✅ Shared dependencies configured
- ✅ Cargo.lock committed for reproducibility

**Development Tooling:**
- ✅ `clippy.toml` - Clippy configuration
- ✅ `rustfmt.toml` - Code formatting
- ✅ `deny.toml` - Dependency auditing
- ✅ `.envrc` - direnv integration
- ✅ `flake.nix` - Nix development shell with SQLCipher
- ✅ `.gitignore` - Proper ignore patterns

**Systemd Integration:**
- ✅ `systemd/dots-family-daemon.service` - systemd unit file
- ✅ `dbus/org.dots.FamilyDaemon.service` - DBus activation

**CI/CD:**
- ✅ `.github/workflows/ci.yml` - GitHub Actions pipeline

**Database:**
- ✅ `migrations/` directory ready for sqlx migrations
- ✅ SQLCipher integration in flake.nix

### Build & Test Status

**Build:** ⚠️ Compiles with some warnings
```bash
cargo build --workspace  # Compiles successfully
cargo clippy --workspace --all-features -- -D warnings  # Has some warnings
```

**Tests:** ⚠️ Partially passing
- `dots-family-common`: Tests needed
- `dots-family-proto`: Tests needed
- `dots-family-db`: 1 test passing (basic)
- `dots-family-daemon`: 4 passing, 3 FAILING (integration tests need daemon)
- Total: ~4-5 tests passing, 3 failing

**Metrics:**
- 39 Rust source files
- ~3,500+ lines of production code (estimate)
- 10 crates configured
- 3 crates fully functional (common, proto, db)
- 3 crates mostly functional (daemon, monitor, ctl)

## Critical Configuration Notes

### 1. Database Setup
- **SQLCipher** integration via sqlx
- **Encryption key**: Not yet implemented (Phase 0 decision: password-derived with Argon2)
- **Migrations**: Directory exists at `migrations/`, ready for sqlx migrations
- **Connection pooling**: Implemented in `dots-family-db`

### 2. DBus Configuration
- **Interface**: `org.dots.FamilyDaemon`
- **Path**: `/org/dots/FamilyDaemon`
- **Bus type**: Session bus (decided in analysis)
- **Activation**: Configured in `dbus/org.dots.FamilyDaemon.service`

### 3. Nix Environment
- **Required**: Must run in `nix develop` shell
- **SQLCipher**: Provided by flake.nix
- **Check**: `echo $IN_NIX_SHELL` should be set

## What's NOT Done Yet (Phase 0 Remaining ~15%)

### Immediate Priorities

1. **Fix Integration Tests** (High Priority)
   - 3 failing tests in `dots-family-daemon`
   - Need either:
     - Better mock daemon implementation
     - Conditional test execution (skip if daemon not running)
   - File: `crates/dots-family-daemon/tests/integration_test.rs`

2. **Add Missing Unit Tests** (High Priority)
   - `dots-family-common`: No tests yet
   - `dots-family-proto`: No tests yet
   - `dots-family-db`: Only 1 basic test

3. **Database Migration Files** (High Priority)
   - Create initial migration in `migrations/`
   - Schema defined in `../dots-detection/docs/DATA_SCHEMA.md`
   - Use `sqlx migrate add initial_schema`

4. **Fix Clippy Warnings** (Medium Priority)
   - Run `cargo clippy --workspace --all-features -- -D warnings`
   - Fix all warnings for clean build

5. **Documentation** (Medium Priority)
   - Add inline documentation to public APIs
   - Create per-crate README.md files
   - Document build and test procedures

## Next Session Quick Start

### Environment Setup
```bash
cd /home/shift/code/endpoint-agent/dots-family-mode
nix develop  # REQUIRED - provides SQLCipher
cargo build --workspace
cargo test --workspace
```

### Development Commands
```bash
# Build specific crate
cargo build -p dots-family-daemon
cargo build -p dots-family-monitor
cargo build -p dots-family-ctl

# Test specific crate
cargo test -p dots-family-daemon
cargo test -p dots-family-db

# Run daemon (requires DBus system bus)
cargo run -p dots-family-daemon

# Run monitor
cargo run -p dots-family-monitor

# Run CLI
cargo run -p dots-family-ctl -- status
cargo run -p dots-family-ctl -- profile list

# Full workspace operations
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-features
```

### Key Files to Review

**For understanding current implementation:**
- `crates/dots-family-common/src/types.rs` - Core data structures
- `crates/dots-family-daemon/src/dbus_impl.rs` - Daemon DBus interface
- `crates/dots-family-db/src/queries/` - Database query layer
- `crates/dots-family-monitor/src/wayland.rs` - Wayland compositor integration

**For fixing failing tests:**
- `crates/dots-family-daemon/tests/integration_test.rs` - Failing integration tests

**For database work:**
- `../dots-detection/docs/DATA_SCHEMA.md` - Complete schema specification (20 tables)
- `migrations/` - Empty, needs initial migration

## Design Documentation Reference

**Location:** `../dots-detection/docs/`

All design documentation is in the sibling project:
- `ARCHITECTURE.md` - System design
- `DATA_SCHEMA.md` - Database schema (20 tables)
- `RUST_APPLICATIONS.md` - Application specifications
- `IMPLEMENTATION_ROADMAP.md` - 10-phase plan
- `IMPLEMENTATION_ANALYSIS.md` - Phase 0 breakdown (this was just created)
- `PARENTAL_CONTROLS.md` - Control mechanisms
- `CONTENT_FILTERING.md` - Filtering design
- `MONITORING.md` - Monitoring features
- `WM_INTEGRATION.md` - Window manager integration
- `TERMINAL_INTEGRATION.md` - Terminal integration

## Phase 0 Completion Checklist

- [x] Create Cargo workspace structure
- [x] Implement dots-family-common (types, errors, config)
- [x] Implement dots-family-proto (DBus interfaces)
- [x] Implement dots-family-db (connection, queries)
- [x] Implement dots-family-daemon (core service)
- [x] Implement dots-family-monitor (activity tracking)
- [x] Implement dots-family-ctl (CLI tool)
- [x] Set up build system and tooling
- [x] Create systemd and DBus integration files
- [x] Set up Nix development environment
- [ ] **Fix failing integration tests** ← NEXT PRIORITY
- [ ] **Add comprehensive unit tests** ← NEXT PRIORITY
- [ ] **Create database migrations** ← NEXT PRIORITY
- [ ] Fix all clippy warnings
- [ ] Document public APIs
- [ ] Verify full workspace build with zero warnings

**Estimated Completion:** 90-95% done, ~4-6 hours remaining

## Phase 1 Preview (After Phase 0)

Once Phase 0 is complete, Phase 1 will focus on:

1. **Monitor → Daemon Integration**
   - Monitor reports window activity to daemon via DBus
   - Daemon stores activity in database
   - Real-time activity tracking working end-to-end

2. **Policy Enforcement**
   - Daemon loads policies from database
   - Policy engine enforces time limits
   - Application blocking functional

3. **Profile Loading**
   - Profiles stored in database
   - Daemon loads active profile on startup
   - Profile switching working

4. **Authentication**
   - Parent password with Argon2 hashing
   - Database encryption key derivation
   - Secure configuration storage

**Estimated Timeline:** 3-4 weeks after Phase 0 completion

## Git Status

**Current Repository:** `dots-family-mode` (misnamed as `dots-detection`)

**Recent Commits:**
```
9dadb9d (HEAD) build: add Cargo.lock for reproducible builds
a6ca72a refactor: fix clippy warnings and improve code quality
81ff883 test: add mock daemon for integration testing
4aa9f89 feat: implement database layer with SQLCipher support
5be4d3b test: add comprehensive tests and development tooling
607ba8d feat: initialize dots-family-mode workspace with Phase 0 foundation
```

**Uncommitted Changes:** None (clean working tree as of last commit)

## Common Pitfalls & Solutions

### 1. Build Failures
**Problem:** `cargo build` fails with SQLCipher errors
**Solution:** Ensure you're in `nix develop` shell (`echo $IN_NIX_SHELL`)

### 2. Integration Tests Fail
**Problem:** Tests expect running daemon
**Solution:** Either run daemon in background or skip with `cargo test --lib`

### 3. DBus Errors
**Problem:** "No such interface" or "Service not found"
**Solution:** Check if system bus is running: `echo $DBUS_SYSTEM_BUS_ADDRESS`

### 4. Clippy Warnings
**Problem:** Many warnings about unused code
**Solution:** This is expected - placeholder crates have minimal code

## Development Philosophy

- **Privacy-first**: All data local, no cloud
- **Security**: Tamper-resistant, encrypted database
- **Cross-WM compatible**: Niri, Sway, Hyprland
- **Production-ready**: Comprehensive error handling and testing
- **Clean code**: Zero clippy warnings with `-D warnings` (goal)

## Questions for Next Session

If starting Phase 0 completion:
1. **Focus area**: Tests, migrations, or documentation?
2. **Integration test strategy**: Mock better or conditional execution?
3. **Database encryption**: Implement now or defer to Phase 1?

If starting Phase 1:
1. **Which integration first**: Monitor→Daemon or Policy enforcement?
2. **Authentication priority**: Implement early or defer?
3. **GUI work**: Start GTK4 dashboard or keep CLI-only for now?

# Engram Workflow Integration - COMPREHENSIVE GUIDE

## What is Engram?

**Engram** is a **Task-driven Memory System for LLM Coding Agents** built in Rust that maintains project state, tasks, and reasoning across coding sessions. It enforces disciplined development via Git commit validation requiring task references.

**Location:** `/home/shift/.nix-profile/bin/engram` (Available as `engram` command)
**Purpose:** Maintains project state, tasks, and reasoning across coding sessions
**Git Integration:** Commits must reference task UUIDs: `"feat: implement auth [<uuid>]"`

## Core Architecture

### Entity System
- **Tasks**: Core work units with UUID identifiers, returns UUIDs for commit references
- **Context**: Background information and documentation 
- **Reasoning**: Decision chains and rationale (REQUIRED for task validation)
- **Relationships**: Link entities (REQUIRED: task↔reasoning, task↔context for validation)
- **Workflows**: State machines and process flows

### Core Workflow (Essential Steps)
```bash
1. engram setup workspace              # Initialize project
2. engram task create --title "..."    # Create work items (returns UUIDs)
3. engram context create --title "..." # Add background info
4. engram reasoning create --task-id <uuid> # Document decisions
5. engram relationship create ...       # Link entities (REQUIRED for validation)
6. engram validate hook install        # Enable Git integration
```

### Storage Architecture
- **Git Integration**: All commits must reference valid task UUIDs
- **JSON I/O**: Most commands support --json input/output for programmatic access
- **Entity Storage**: `.engram/` directory (separate from main project)
- **Validation**: Pre-commit hooks enforce task references

## Session Startup Protocol (MANDATORY)

Every session MUST begin with this protocol:

### 1. Check Engram Availability
```bash
# Verify engram is available
engram help 2>/dev/null || echo "Engram not found in PATH"
```

### 2. Review Current Tasks
```bash
# List all pending tasks
engram task list

# Find your active/assigned tasks  
engram task list --agent default

# Check tasks in JSON format for programmatic access
engram task list --agent default | jq '.[].id'
```

### 3. Check Validation Status
```bash
# Check validation setup
engram validate hook status

# Install hook if missing
engram validate hook install

# Test commit validation (dry-run)
engram validate commit --message "test commit [<uuid>]" --dry-run
```

### 4. Check Workspace Status
```bash
# Verify workspace is initialized
ls .engram/ 2>/dev/null || engram setup workspace
```

## Complete Command Reference

### Setup Commands
```bash
# Initialize workspace (do this once per project)
engram setup workspace

# Check current workspace status
ls .engram/
```

### Task Management (Core Entity)
```bash
# Create new task (saves UUID for commit references)
TASK_ID=$(engram task create --title "Implement feature X" --json | jq -r '.id')
engram task create --title "Fix database issues" --priority high

# List tasks with filtering
engram task list
engram task list --agent default
engram task list --json  # JSON output for programmatic access

# Get task details
engram task get <task-id>

# Update task (status changes, metadata)
engram task update <task-id> --status completed
engram task update <task-id> --priority high

# Create task from JSON input
echo '{"title": "Database migration", "priority": "high"}' | engram task create --json
```

### Context Management (Background Info)
```bash
# Create context entities
CTX_ID=$(engram context create --title "Feature requirements" --source "requirements.md" --json | jq -r '.id')
engram context create --title "Technical constraints" --description "System limitations"

# List contexts
engram context list
```

### Reasoning Management (Decision Documentation)
```bash
# Create reasoning (REQUIRED - links to tasks for validation)
REASON_ID=$(engram reasoning create --task-id <task-id> --title "Implementation approach" --json | jq -r '.id')
engram reasoning create --task-id <task-id> --title "Alternative evaluation" --description "Why we chose X over Y"

# List reasoning
engram reasoning list
```

### Relationship Management (CRITICAL for Validation)
```bash
# Create task → context relationship (REQUIRED)
engram relationship create \
  --source-id <task-id> --source-type task \
  --target-id <context-id> --target-type context \
  --relationship-type references --agent default

# Create task → reasoning relationship (REQUIRED)  
engram relationship create \
  --source-id <task-id> --source-type task \
  --target-id <reasoning-id> --target-type reasoning \
  --relationship-type references --agent default

# Create task hierarchies (parent → subtask)
engram relationship create \
  --source-id <parent-task> --source-type task \
  --target-id <subtask> --target-type task \
  --relationship-type contains --agent default

# Create task dependencies
engram relationship create \
  --source-id <task2> --source-type task \
  --target-id <task1> --target-type task \
  --relationship-type depends_on --agent default

# Query relationships
engram relationship connected --entity-id <task-id> --relationship-type references
engram relationship find-path --source-id <task-id> --target-id <context-id>
```

### Validation System (Git Integration)
```bash
# Install validation hooks (required for commit enforcement)
engram validate hook install

# Check hook status
engram validate hook status

# Test commit validation (dry-run)
engram validate commit --message "feat: implement auth [<task-id>]" --dry-run

# Check validation issues
engram validate check
```

### Workflow Management
```bash
# Create workflows (state machines)
engram workflow create --title "Feature development workflow" --description "Multi-stage development process"

# List workflows
engram workflow list

# Check workflow status
engram workflow status --task-id <task-id>
```

## Task Execution Workflow (MANDATORY PROCESS)

### Complete Example Workflow
```bash
# 1. SETUP (once per project)
engram setup workspace

# 2. CREATE ENTITIES WITH UUIDs
TASK_ID=$(engram task create --title "Add OAuth support" --json | jq -r '.id')
CTX_ID=$(engram context create --title "OAuth 2.0 specification" --source "RFC 6749" --json | jq -r '.id')
REASON_ID=$(engram reasoning create --task-id $TASK_ID --title "Why OAuth over custom auth" --json | jq -r '.id')

# 3. CREATE REQUIRED RELATIONSHIPS (VALIDATION REQUIREMENT)
engram relationship create \
  --source-id $TASK_ID --source-type task \
  --target-id $CTX_ID --target-type context \
  --relationship-type references --agent default

engram relationship create \
  --source-id $TASK_ID --source-type task \
  --target-id $REASON_ID --target-type reasoning \
  --relationship-type references --agent default

# 4. ENABLE VALIDATION
engram validate hook install

# 5. WORK ON TASK
# ... do development work ...

# 6. COMMIT WITH TASK REFERENCE
git commit -m "feat: add OAuth endpoint [$TASK_ID]"

# 7. UPDATE TASK STATUS
engram task update $TASK_ID --status completed
```

### Before Starting Any Work

1. **Initialize if needed**:
```bash
# Check if workspace exists
ls .engram/ 2>/dev/null || engram setup workspace
```

2. **Check existing tasks**:
```bash
# List all tasks
engram task list

# Find specific tasks
engram task list --json | jq '.[] | select(.status != "completed")'
```

3. **Verify validation setup**:
```bash
engram validate hook status
```

### During Development

1. **Create proper entity structure for any new work**:
```bash
# Every task MUST have context and reasoning for validation
TASK_ID=$(engram task create --title "Your task" --json | jq -r '.id')
CTX_ID=$(engram context create --title "Task context" --json | jq -r '.id') 
REASON_ID=$(engram reasoning create --task-id $TASK_ID --title "Approach" --json | jq -r '.id')

# Link them (REQUIRED)
engram relationship create --source-id $TASK_ID --source-type task --target-id $CTX_ID --target-type context --relationship-type references --agent default
engram relationship create --source-id $TASK_ID --source-type task --target-id $REASON_ID --target-type reasoning --relationship-type references --agent default
```

2. **Follow commit validation**:
   - All commits MUST reference valid task UUIDs: `"feat: implement auth [<uuid>]"`
   - Test with: `engram validate commit --message "your message [<uuid>]" --dry-run`

### Task Completion

1. **Verify relationships exist**:
```bash
engram relationship connected --entity-id <task-id> --relationship-type references
```

2. **Update task status**:
```bash
engram task update <task-id> --status completed
```

## Complex Task Structure Patterns

### Creating Task Hierarchies
```bash
# Create parent task
PARENT_TASK=$(engram task create --title "Implement authentication system" --json | jq -r '.id')

# Create subtasks  
TASK1=$(engram task create --title "Design auth schema" --json | jq -r '.id')
TASK2=$(engram task create --title "Implement JWT handling" --json | jq -r '.id')
TASK3=$(engram task create --title "Add auth middleware" --json | jq -r '.id')

# Create containment relationships
engram relationship create --source-id $PARENT_TASK --source-type task --target-id $TASK1 --target-type task --relationship-type contains --agent default
engram relationship create --source-id $PARENT_TASK --source-type task --target-id $TASK2 --target-type task --relationship-type contains --agent default
engram relationship create --source-id $PARENT_TASK --source-type task --target-id $TASK3 --target-type task --relationship-type contains --agent default

# Create dependencies (logical order)
engram relationship create --source-id $TASK2 --source-type task --target-id $TASK1 --target-type task --relationship-type depends_on --agent default
engram relationship create --source-id $TASK3 --source-type task --target-id $TASK2 --target-type task --relationship-type depends_on --agent default
```

## Finding and Resuming Work

### When Starting a Session:

1. **Check recent activity**:
```bash
# See all current tasks
engram task list

# Get tasks in JSON for programmatic filtering
engram task list --json | jq '.[] | select(.status != "completed") | .id'
```

2. **Find incomplete work**:
```bash
# Find tasks by status
engram task list --json | jq '.[] | select(.status == "in_progress")'

# Find tasks by priority  
engram task list --json | jq '.[] | select(.priority == "high")'
```

3. **Review relationships**:
```bash
# Check what entities are connected to a task
engram relationship connected --entity-id <task-id>

# Find path between entities
engram relationship find-path --source-id <task1> --target-id <task2>
```

## Current Project Integration

### Project Configuration
The project has basic engram structure in `.engram/config.yaml`:
```yaml
agents:
  coder:
    agent_type: implementation
  planner:
    agent_type: architecture  
  reviewer:
    agent_type: quality_assurance
```

### Current Task Status
**ENGRAM IS THE SOURCE OF TRUTH**: Check engram for current task status:

```bash
# Check current state
engram task list

# Find active work
engram task list | grep -E "(inprogress|todo)"

# Get task details
engram task get <task-id>
```

**Setup Required**: Validation hooks must be installed before starting work:
```bash
engram validate hook install
engram validate check
```

### Current Validation Status
```bash
$ engram validate check
⚠️  Issues found:
  • Pre-commit hook not installed
  • Validation not working

Run 'engram validation hook install' to fix setup issues.
```

**Setup Required**: Validation hooks must be installed before starting work.

### Documentation Integration
The `../dots-detection/docs/` directory contains 289 uncompleted tasks marked with `[ ]` checkboxes:
- **IMPLEMENTATION_ROADMAP.md**: 10 phases with detailed task breakdowns
- **SECURITY_ARCHITECTURE.md**: Security implementation tasks  
- **IMPLEMENTATION_ANALYSIS.md**: Phase analysis tasks
- **REVIEW_AND_PRIORITIES.md**: Architecture and system service tasks

**Creating tasks from docs**: Use `grep -r "\[ \]" docs/` to find specific tasks to add to engram.

## Troubleshooting

### Common Issues and Solutions

**Hook Not Working**:
```bash
engram validate hook status
engram validate hook install
```

**Task Missing Relationships** (Validation will catch this):
```bash
# Check existing relationships
engram relationship connected --entity-id <task-id>

# Create missing relationships (REQUIRED for validation)
engram relationship create --source-id <task-id> --source-type task --target-id <context-id> --target-type context --relationship-type references --agent default
```

**Workspace Not Initialized**:
```bash
# Initialize workspace if missing
ls .engram/ 2>/dev/null || engram setup workspace
```

**Commit Validation Failing**:
```bash
# Test your commit message format
engram validate commit --message "feat: your change [<valid-task-uuid>]" --dry-run

# Ensure task has required relationships
engram relationship connected --entity-id <task-id>
```

## Agent Handoff Protocol

### For Next Agent Working on This Project:

1. **Environment Check**:
```bash
cd /home/shift/code/endpoint-agent/dots-detection/dots-familt-mode
echo $IN_NIX_SHELL  # Should be set
engram task list  # Check current tasks
```

2. **Setup Validation** (if not done):
```bash
# Install hooks if not present
engram validate hook install

# Verify setup
engram validate hook status
```

3. **Review Context**:
- Main project: Phase 0 mostly complete (85%), needs integration tests fixed
- Family mode: `dots-familt-mode/` codebase with compilation issues
- Documentation: `docs/` comprehensive specs and roadmap  
- Current priority: Fix failing integration tests, database migrations

4. **Create Tasks for Immediate Work**:
```bash
# Create task for next priority work
TASK_ID=$(engram task create --title "Fix failing integration tests" --priority high --json | jq -r '.id')

# Create context and reasoning
CTX_ID=$(engram context create --title "Integration test context" --description "Tests expect daemon running or better mocks" --json | jq -r '.id')
REASON_ID=$(engram reasoning create --task-id $TASK_ID --title "Test fix approach" --description "Either improve mocks or conditional execution" --json | jq -r '.id')

# Create required relationships
engram relationship create --source-id $TASK_ID --source-type task --target-id $CTX_ID --target-type context --relationship-type references --agent default
engram relationship create --source-id $TASK_ID --source-type task --target-id $REASON_ID --target-type reasoning --relationship-type references --agent default

# Start work
engram task update $TASK_ID --status in_progress
```

5. **Track Progress**:
- Update task status immediately when status changes
- Create new tasks for any additional work discovered
- Maintain proper entity relationships throughout development  
- Follow commit validation with task ID references

## Critical Rules (NON-NEGOTIABLE)

1. **ALWAYS use engram for task tracking** - If it's not in engram, it doesn't exist
2. **Every task MUST have context and reasoning relationships** for validation to pass
3. **All commits must reference valid task UUIDs** with format: `"feat: your change [uuid]"`  
4. **Install validation hooks** before starting any development work
5. **Use JSON output for programmatic access** when creating entities that need UUID capture
6. **Create proper entity structures** before starting work, not after
7. **Test commit messages** with `--dry-run` before actual commits

## Quick Reference

### Essential Commands
```bash
# Session startup
engram task list
engram validate hook status

# Create work with proper structure
TASK_ID=$(engram task create --title "..." --json | jq -r '.id')
CTX_ID=$(engram context create --title "..." --json | jq -r '.id')
REASON_ID=$(engram reasoning create --task-id $TASK_ID --title "..." --json | jq -r '.id')

# Create required relationships
engram relationship create --source-id $TASK_ID --source-type task --target-id $CTX_ID --target-type context --relationship-type references --agent default
engram relationship create --source-id $TASK_ID --source-type task --target-id $REASON_ID --target-type reasoning --relationship-type references --agent default

# Validate and commit
engram validate commit --message "feat: your change [$TASK_ID]" --dry-run
git commit -m "feat: your change [$TASK_ID]"
```

### Storage Locations
- **Engram Binary**: `/home/shift/.nix-profile/bin/engram` (available as `engram` command)
- **Project Config**: `.engram/config.yaml`
- **Entity Storage**: `.engram/` directory
- **Validation**: Pre-commit hooks in `.git/hooks/`

Remember: Engram enforces disciplined development. Always use engram commands to discover current state, create proper task structures with relationships, and reference task UUIDs in commits. The system ensures nothing falls through the cracks.
