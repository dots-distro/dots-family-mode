# VM Testing Strategy - End-to-End Approval Workflow

## Overview

This document outlines the strategy for testing the DOTS Family Mode approval workflow in a NixOS VM. We have two test files ready, but we need to understand how to execute them and what to expect.

## Current State Analysis

### Test Files We Have

1. **test-approval-workflow.nix** (171 lines)
   - Basic infrastructure test
   - Tests daemon startup, CLI, DBus registration
   - No authentication or actual workflow testing
   - 10 test scenarios covering basics

2. **test-approval-integration.nix** (191 lines) 
   - More comprehensive integration test
   - Requires NixOS module import
   - Tests profile creation, GUI launch, DBus signals
   - Has placeholders for actual approval workflow

### What Tests Currently Do

**test-approval-workflow.nix Tests:**
1. Package installation verification
2. Daemon startup via systemd
3. Daemon log inspection
4. CLI command availability
5. GUI binary existence
6. DBus interface registration
7. Database creation
8. Infrastructure validation
9. Resource usage monitoring
10. Graceful shutdown

**test-approval-integration.nix Tests:**
1. Child profile creation
2. Empty approval request listing
3. Approval request creation (simulated)
4. CLI authentication
5. GUI launch verification
6. DBus signal listening
7. Approval deny (placeholder)
8. Approval approve (placeholder)
9. Daemon log inspection
10. DBus policy verification

## Problems Identified

### Issue 1: Missing NixOS Module Reference
**File:** `test-approval-integration.nix` line 8
```nix
imports = [ ../../nixos-modules/dots-family/default.nix ];
```

**Problem:** This path doesn't exist in our repo structure.

**Solution Options:**
1. Skip this test and focus on test-approval-workflow.nix
2. Remove the module import and configure manually
3. Create a minimal module file for testing

**Recommendation:** Option 1 - Focus on simpler workflow test first.

### Issue 2: No Actual Approval Request Creation

**Problem:** Both tests have placeholders for creating actual approval requests. The daemon needs a method to programmatically create requests.

**Solution Options:**
1. Add a DBus method to create test requests
2. Manually insert test data into the database
3. Create requests via CLI (need to add command)
4. Focus on infrastructure testing first

**Recommendation:** Option 4 - Validate infrastructure, then enhance.

### Issue 3: Authentication Requirements

**Problem:** CLI commands require password authentication via stdin:
```bash
echo 'testpass' | dots-family-ctl approval list
```

**Verification Needed:**
- Does the CLI actually accept password via stdin?
- Is the password hash in the test correct?
- Does the daemon validate passwords properly?

**Solution:** Test with the simpler workflow test first.

### Issue 4: DBus Service Name Uncertainty

**Problem:** Don't know the exact DBus service name the daemon registers.

**Possible Names:**
- `org.dots.FamilyDaemon`
- `org.dots.family.Daemon`
- `ai.dots.FamilyDaemon`
- Something else

**Solution:** Check daemon source code or run test to discover.

## Execution Strategy

### Phase 1: Infrastructure Test (FOCUS HERE FIRST)

**Goal:** Validate basic daemon and CLI work in VM

**Command:**
```bash
nix build .#checks.x86_64-linux.approval-workflow-test -L
```

**Expected Time:** 10-15 minutes (first run)

**Success Criteria:**
- VM boots successfully
- Daemon starts and runs
- CLI binaries are installed
- DBus service registers
- Database file is created
- No panics or crashes

**Likely Issues:**
1. DBus service name mismatch - Test checks for "org.dots" but actual name differs
2. Database not created - Daemon may need trigger to init DB
3. Daemon crashes on startup - Missing dependencies or configuration

### Phase 2: Fix Infrastructure Issues

Based on Phase 1 results:

**If DBus name is wrong:**
1. Check daemon logs for actual service name
2. Update test script with correct name
3. Re-run test

**If database doesn't exist:**
1. Add test step to trigger DB creation (call a CLI command)
2. Or accept that DB is created on first use
3. Update test expectations

**If daemon crashes:**
1. Review daemon logs in test output
2. Check for missing dependencies
3. Add required packages to VM config
4. Fix daemon code if needed

### Phase 3: Enhanced Workflow Test (FUTURE)

Once infrastructure works, create end-to-end test:

**Test Scenario:**
1. Create parent profile with password
2. Create child profile
3. Programmatically create approval request
4. List requests via CLI
5. Approve request via CLI
6. Verify request status changed
7. Deny a request via CLI
8. Verify request denied

**Requires:**
- Working authentication
- Method to create test requests
- Database query validation

## Detailed Execution Plan

### Step 1: Check Current Environment

```bash
# Verify we're not in devShell (for consistency)
env | grep IN_NIX_SHELL

# Check git status
git status

# Verify test files exist
ls -la tests/nix/test-approval-*
```

### Step 2: Attempt First Test Run

```bash
# Run the simpler infrastructure test with verbose logging
nix build .#checks.x86_64-linux.approval-workflow-test -L 2>&1 | tee vm-test-run1.log
```

**What to Watch For:**
- Build progress (200+ dependencies)
- Compilation errors
- Test execution output
- Pass/fail status

### Step 3: Analyze Test Output

The test output will show:
- Each test section with ✅ or ⚠️
- Daemon logs
- DBus service list
- Database status
- Error messages

**Expected Output Format:**
```
=== Test 1: Package Installation ===
✅ All binaries are installed

=== Test 2: Daemon Startup ===
✅ Daemon is running

...etc
```

### Step 4: Handle Common Failures

**Failure A: Test compilation fails**
- Check for syntax errors in test file
- Verify pkgs and self are properly passed
- Check imports are valid

**Failure B: VM won't boot**
- Check NixOS configuration is valid
- Verify systemd services are correctly defined
- Check for conflicting services

**Failure C: Daemon won't start**
- Check daemon binary exists
- Verify dependencies are in environment
- Review daemon startup requirements
- Check database path permissions

**Failure D: DBus not working**
- Verify dbus.service is enabled
- Check DBus policy files are installed
- Verify daemon registers on correct bus (system vs session)

**Failure E: Tests timeout**
- Increase sleep times in test
- Check if daemon actually started
- Look for hung processes

### Step 5: Fix and Iterate

Based on failures:
1. Update test configuration
2. Fix daemon issues
3. Adjust test expectations
4. Re-run test

## Alternative: Manual VM Testing

If automated tests are too complex initially, we can test manually:

### Manual Test Approach

**Step 1: Build a test VM**
```bash
nix build .#checks.x86_64-linux.approval-workflow-test
```

**Step 2: Run VM interactively**
```bash
# This might work if nixosTest exposes it
./result/bin/run-*-vm
```

**Step 3: Inside VM, manually test:**
```bash
# Check daemon
systemctl status dots-family-daemon-test

# Check logs
journalctl -u dots-family-daemon-test -f

# Test CLI
dots-family-ctl --help
dots-family-ctl approval list

# Check DBus
dbus-send --system --print-reply --dest=org.freedesktop.DBus \
  /org/freedesktop/DBus org.freedesktop.DBus.ListNames

# Check database
ls -la /var/lib/dots-family/
sqlite3 /var/lib/dots-family/family.db ".tables"
```

## Questions to Answer During Testing

1. What is the actual DBus service name the daemon registers?
2. Does the database get created automatically or need initialization?
3. Does CLI password authentication work via stdin?
4. Are there any missing runtime dependencies?
5. Does the daemon stay running or crash after startup?
6. What's the actual memory/CPU footprint?
7. Are logs helpful for debugging?
8. Does graceful shutdown work properly?

## Success Metrics

### Minimum Success (Phase 1)
- VM boots and reaches multi-user.target
- DBus service starts
- Daemon binary executes
- CLI binary exists and shows help
- No panics or segfaults

### Good Success (Phase 2)
- Daemon stays running for duration of test
- DBus service registers (any name)
- Database file is created
- CLI commands execute (even if they fail auth)
- Graceful shutdown works

### Full Success (Phase 3)
- All infrastructure tests pass
- Can create profiles via CLI
- Can list approval requests
- Authentication works
- Can approve/deny requests (when they exist)
- No errors in daemon logs

## Timeline Estimate

**Phase 1 (Infrastructure Test):**
- Initial run: 15 minutes
- Analyze results: 15 minutes
- Fix issues: 30 minutes
- Re-test: 10 minutes
**Total: ~70 minutes**

**Phase 2 (Fix Issues):**
- Depends on what breaks
- Could be 30 minutes to 2 hours
**Total: ~30-120 minutes**

**Phase 3 (Enhanced Testing):**
- Future session
**Total: 2-4 hours**

## Next Actions

1. Run the infrastructure test: `nix build .#checks.x86_64-linux.approval-workflow-test -L`
2. Capture full output to a log file
3. Analyze which tests pass and which fail
4. Create fixes for any failures
5. Document actual behavior vs expected
6. Iterate until infrastructure test passes

## Notes and Observations

### DBus Complexity
DBus testing is tricky because:
- Service might register under different name
- Policy files need to be in correct location
- System bus vs session bus confusion
- Timing issues (daemon might not register immediately)

### Database Initialization
We don't know if the daemon:
- Creates DB on startup
- Creates DB on first API call
- Requires explicit initialization
- Uses migrations

### Authentication Flow
Need to verify:
- Password hash format matches what daemon expects
- CLI actually reads from stdin
- Daemon validates against environment variable or config file

### Resource Usage
The test measures memory and CPU, but we don't have baselines for:
- Normal memory usage
- Expected CPU percentage
- Whether these metrics indicate problems

## Conclusion

Start with the simpler infrastructure test (`test-approval-workflow.nix`) to validate basic functionality. This test is more self-contained and doesn't depend on external modules. Once this passes, we can enhance it to test actual approval workflows.

The key is to iterate quickly:
1. Run test
2. Identify failure
3. Fix one thing
4. Re-run test

Don't try to fix everything at once. Focus on getting the daemon running first, then worry about DBus, then database, then CLI functionality.
