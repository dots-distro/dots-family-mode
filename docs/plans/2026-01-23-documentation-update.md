# Documentation Update Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Update `README.md` and related documentation to accurately reflect the current project structure, testing status, and recent architectural changes (reorganization of `crates/`, `scripts/`, etc.).

**Architecture:** We will systematically update the primary entry point (`README.md`), `PROJECT_STATUS.md` (which serves as a dynamic status tracker), and check `docs/INDEX.md` for broken links or outdated references caused by the recent moves.

**Tech Stack:** Markdown, git.

### Task 1: Update README.md - Structure and Status

**Files:**
- Modify: `README.md`

**Step 1: Update Project Structure Section**
Update the "Project Structure" section to match the current reality.
- `crates/` (verify all sub-crates are listed)
- `scripts/` (update subdirectories like `tests/`, `setup/`)
- Remove references to root-level folders that were moved or deleted (e.g., if `migrations_backup/` was mentioned).

**Step 2: Update Testing Status & Limitations**
Add a "Known Limitations" or update the "Current Status" section in `README.md`.
- Explicitly state that Browser Tests via Playwright are **limited in the NixOS development environment** due to browser binary issues.
- Recommend using the VM environment for full browser testing.

**Step 3: Commit**
```bash
git add README.md
git commit -m "docs: update README with new structure and testing limitations"
```

### Task 2: Update PROJECT_STATUS.md

**Files:**
- Modify: `PROJECT_STATUS.md`

**Step 1: Reflect Current State**
- Update the "Browser Tests (Dev)" status to "Limited / Manual Fallback".
- Update "Browser Tests (VM)" to "Ready for Verification".
- Ensure the "Recent Changes" section reflects the repo reorganization and Nix fixes.

**Step 2: Commit**
```bash
git add PROJECT_STATUS.md
git commit -m "docs: update PROJECT_STATUS with latest testing status"
```

### Task 3: Verify and Update Docs Index

**Files:**
- Modify: `docs/INDEX.md` (if necessary)

**Step 1: Check for Broken References**
- Quickly scan `docs/INDEX.md` and `docs/VM_TESTING_GUIDE.md` (if it exists) to ensure they don't point to old paths like `./src/` instead of `./crates/`.

**Step 2: Fix Paths**
- Update any paths that point to `src/*` to `crates/*`.
- Update script paths to `scripts/*`.

**Step 3: Commit**
```bash
git add docs/
git commit -m "docs: fix file paths in documentation index"
```

### Task 4: Final Verification

**Step 1: Render check**
- (Self-check) Ensure markdown formatting is correct.

**Step 2: Task Completion**
- Update Engram task status to done.
