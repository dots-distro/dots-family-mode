# Repository Cleanup and Reorganization Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Clean up the repository root, consolidate related files, and establish a clear directory structure for deployment, testing, and source code.

**Architecture:**
- `crates/`: All Rust source code.
- `deployment/`: System configuration (systemd, dbus, ebpf policies).
- `scripts/`: Helper scripts, categorized by function (test, setup, legacy).
- `tests/nix/`: All Nix-based test configurations and VM definitions.
- `integration/`: Shell integrations.

**Tech Stack:** Git, Bash

---

### Task 1: Artifact Cleanup and .gitignore

**Files:**
- Modify: `.gitignore`
- Delete: `scripts/web-filtering-test/node_modules/`, `test-evidence/`, `logs/`, `target/` (from git index)

**Step 1: Update .gitignore**
Add/Verify these lines in `.gitignore`:
```
# Build Artifacts
target/
result
result-*
node_modules/

# Test Output
test-evidence/
logs/
*.log
*.qcow2
*.db
sqlite:
```

**Step 2: Remove artifacts from Git Index**
We accidentally committed these. We need to remove them from git tracking without deleting the files from disk (unless they are truly garbage).
```bash
git rm -r --cached scripts/web-filtering-test/node_modules/
git rm -r --cached test-evidence/
git rm -r --cached logs/
# If target/ was committed:
git rm -r --cached target/ || true
```

**Step 3: Commit**
```bash
git commit -m "chore: remove build artifacts and logs from git"
```

---

### Task 2: Reorganize Scripts Directory

**Files:**
- Move: `scripts/*` -> `scripts/{tests,setup,ci,legacy}/`

**Step 1: Create categories**
```bash
mkdir -p scripts/tests scripts/setup scripts/ci scripts/legacy
```

**Step 2: Move Setup/Dev Scripts**
```bash
mv scripts/setup-dev-env.sh scripts/setup/
mv scripts/install-services.sh scripts/setup/
mv scripts/setup-vm.sh scripts/setup/
mv scripts/cargo-with-ebpf.sh scripts/setup/
# Add others as identified
```

**Step 3: Move Test Scripts (from root scripts)**
```bash
mv scripts/vm* scripts/tests/
mv scripts/test* scripts/tests/
mv scripts/run-all-vm-tests.sh scripts/tests/
mv scripts/quick_e2e_test.sh scripts/tests/
```

**Step 4: Move Legacy/Unsure Scripts**
```bash
mv scripts/*.sh scripts/legacy/ 2>/dev/null || true
# Move back any that were wrongly categorized if needed
```

**Step 5: Commit**
```bash
git add scripts/
git commit -m "refactor: reorganize scripts into categories"
```

---

### Task 3: Consolidate Testing Directory

**Files:**
- Move: `testing/scripts/*` -> `scripts/tests/`
- Delete: `testing/` (if empty)

**Step 1: Move testing scripts**
```bash
mv testing/scripts/* scripts/tests/
```

**Step 2: Cleanup testing dir**
If `testing/configs` exists, move it to `tests/configs` or `deployment/configs`? Let's put it in `tests/configs` for now.
```bash
mkdir -p tests/configs
mv testing/configs/* tests/configs/
rmdir testing/scripts testing/configs testing 2>/dev/null || true
```

**Step 3: Commit**
```bash
git add testing/ scripts/ tests/
git commit -m "refactor: consolidate testing scripts and configs"
```

---

### Task 4: Consolidate Nix Configuration

**Files:**
- Move: `nix/*` -> `tests/nix/`
- Move: `tests/*.nix` -> `tests/nix/`
- Update: `flake.nix` (references to these files will break!)

**Step 1: Create directory**
```bash
mkdir -p tests/nix
```

**Step 2: Move files**
```bash
git mv nix/* tests/nix/
git mv tests/*.nix tests/nix/
rmdir nix
```

**Step 3: Update flake.nix (Manual fix required)**
We need to grep for `./nix/` and `./tests/` in `flake.nix` and `default.nix` and update paths.
```bash
# Verify patterns first
grep -r "nix/" flake.nix
grep -r "tests/" flake.nix
# Apply edits (Use Edit tool)
```

**Step 4: Commit**
```bash
git add .
git commit -m "refactor: consolidate nix test configs into tests/nix"
```

---

### Task 5: Root Directory Cleanup (Deployment & Crates)

**Files:**
- Move: `systemd/`, `dbus/`, `dbus-policies/`, `ebpf-config/`, `security-hardening/` -> `deployment/`
- Move: `dots-family-ebpf/` -> `crates/`
- Move: `shell-integration/` -> `integration/`
- Move: `production-testing/` -> `tests/production/`

**Step 1: Create deployment directory**
```bash
mkdir -p deployment
```

**Step 2: Move deployment files**
```bash
git mv systemd deployment/
git mv dbus deployment/
git mv dbus-policies deployment/
git mv ebpf-config deployment/
git mv security-hardening deployment/
```

**Step 3: Move integration**
```bash
mkdir -p integration
git mv shell-integration integration/shell
```

**Step 4: Move crates**
```bash
git mv dots-family-ebpf crates/
# Need to update Cargo.toml workspace members!
```

**Step 5: Move production tests**
```bash
mkdir -p tests/production
git mv production-testing/* tests/production/
rmdir production-testing
```

**Step 6: Fix Cargo.toml**
Update workspace members to include `crates/dots-family-ebpf`.

**Step 7: Commit**
```bash
git add .
git commit -m "refactor: group root directories into deployment, integration, and crates"
```

---

### Task 6: Final Verification

**Step 1: Check structure**
```bash
tree -L 2 -I 'target|node_modules'
```

**Step 2: Verify Build (Basic)**
```bash
cargo check --workspace
```
