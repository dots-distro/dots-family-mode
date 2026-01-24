# DOTS Family Mode - Testing Checklist

## Pre-Testing Setup

### 1. Build All Components
```bash
# Enter development environment
nix develop

# Build daemon
nix build .#dots-family-daemon

# Build CLI
cargo build --package dots-family-ctl

# Build GUI
cargo build --package dots-family-gui
```

### 2. Start Daemon
```bash
# Run daemon in background
./result/bin/dots-family-daemon &

# Verify it's running
ps aux | grep dots-family-daemon
```

### 3. Create Test Profile
```bash
# Create a test child profile
dots-family-ctl profile create testchild "Test Child" --age 10

# Verify profile exists
dots-family-ctl profile list
```

## Approval Requests Testing

### CLI Testing

#### Test 1: List Pending Requests
```bash
dots-family-ctl approval list
```
**Expected:**
- Prompts for parent password
- Shows list of pending requests (or empty if none)
- Each request shows: ID, profile, type, details, timestamp

#### Test 2: Approve a Request
```bash
# First create a test request (if possible via API)
# Then approve it
dots-family-ctl approval approve <request-id> -m "Approved for testing"
```
**Expected:**
- Prompts for parent password
- Confirms approval
- Request removed from pending list

#### Test 3: Deny a Request
```bash
dots-family-ctl approval deny <request-id> -m "Denied for testing"
```
**Expected:**
- Prompts for parent password
- Confirms denial
- Request removed from pending list

#### Test 4: Authentication Failures
```bash
# Enter wrong password when prompted
dots-family-ctl approval list
```
**Expected:**
- Shows "Authentication failed" error
- Does not list requests
- Exits cleanly

### GUI Testing

#### Test 5: Launch GUI
```bash
cargo run --package dots-family-gui
```
**Expected:**
- Window opens with sidebar
- Shows profile list
- All navigation buttons visible

#### Test 6: Access Approval Requests
1. Click "Approval Requests" button in sidebar
2. Should show authentication dialog

**Expected:**
- Authentication dialog appears
- Password field is focused
- "Authenticate" button is disabled when password empty

#### Test 7: Failed Authentication
1. Enter wrong password
2. Click "Authenticate" or press Enter

**Expected:**
- Error message appears: "Authentication failed"
- Password field is cleared
- Can try again

#### Test 8: Successful Authentication
1. Enter correct parent password
2. Click "Authenticate" or press Enter

**Expected:**
- Dialog disappears
- Main approval view appears
- Shows request count or empty state

#### Test 9: Empty State
(When no pending requests)

**Expected:**
- Green checkmark icon
- "No Pending Requests" title
- "All approval requests have been handled" message

#### Test 10: Request List Display
(When requests exist)

**Expected:**
- Shows "X pending request(s)" title
- Each request shows:
  - Profile name (bold)
  - Request type (dimmed)
  - Details text (wrapped)
  - Timestamp
  - "Review" button

#### Test 11: Select Request
1. Click "Review" button on a request card

**Expected:**
- Response message field appears at bottom
- Approve and Deny buttons become visible
- Selected request is highlighted (if implemented)

#### Test 12: Approve Request
1. Select a request
2. (Optional) Enter response message
3. Click "Approve" button

**Expected:**
- Success message appears
- Request disappears from list
- Request count decreases
- Response message field is cleared

#### Test 13: Deny Request
1. Select a request
2. (Optional) Enter response message
3. Click "Deny" button

**Expected:**
- Success message appears
- Request disappears from list
- Request count decreases
- Response message field is cleared

#### Test 14: Refresh Requests
1. Click refresh button (circular arrow icon)

**Expected:**
- List updates with current pending requests
- No visual glitches
- Maintains authenticated state

## Other GUI Views Testing

### Test 15: Dashboard View
1. Click "Dashboard" in sidebar

**Expected:**
- Shows activity overview
- Charts render correctly
- No crashes

### Test 16: Reports View
1. Click "Reports" in sidebar

**Expected:**
- Shows report interface
- Date selection works
- No crashes

### Test 17: Content Filtering View
1. Click "Content Filtering" in sidebar

**Expected:**
- Shows filtering interface
- Blocklist/allowlist visible
- No crashes

### Test 18: Policy Config View
1. Click "Policy Config" in sidebar

**Expected:**
- Shows policy editor
- Time limits visible
- No crashes

### Test 19: Profile Editor View
1. Click profile in sidebar
2. Edit profile settings

**Expected:**
- Shows profile editor
- Can modify settings
- No crashes

## Integration Testing

### Test 20: Child Creates Request → Parent Approves
1. As child: Request access to something (via daemon API)
2. As parent: Open GUI approval view
3. Approve the request
4. Verify child receives approval

**Expected:**
- Request appears in GUI
- Approval is processed
- Child can now access requested resource

### Test 21: Multiple Profiles
1. Create multiple child profiles
2. Create requests from different profiles
3. View in GUI

**Expected:**
- All requests visible
- Profile names displayed correctly
- Can approve/deny each independently

### Test 22: Long-Running Session
1. Open GUI
2. Authenticate once
3. Use for extended period
4. Verify token doesn't expire unexpectedly

**Expected:**
- No repeated authentication prompts
- All operations continue working
- Graceful handling if token expires

## Edge Cases

### Test 23: Daemon Not Running
1. Stop daemon
2. Try to open approval requests in GUI

**Expected:**
- Error message about daemon connection
- Doesn't crash
- Graceful degradation

### Test 24: Network Delays
1. Add artificial delay to daemon responses
2. Test approval operations

**Expected:**
- UI remains responsive
- Loading indicators (if implemented)
- Operations complete successfully

### Test 25: Very Long Request Details
1. Create request with very long details text
2. View in GUI

**Expected:**
- Text wraps properly
- Card doesn't overflow
- Remains readable

### Test 26: Special Characters
1. Create request with special characters in details
2. Approve with special characters in response message

**Expected:**
- Renders correctly
- No encoding issues
- Backend handles properly

## Bug Tracking

### Found Issues
_(Record any issues discovered during testing)_

1. **Issue:** _Description_
   - **Steps to Reproduce:**
   - **Expected Behavior:**
   - **Actual Behavior:**
   - **Severity:** High/Medium/Low
   - **Status:** Open/Fixed

2. _(Add more as needed)_

## Test Results Summary

### CLI Tests
- [ ] Test 1: List requests
- [ ] Test 2: Approve request
- [ ] Test 3: Deny request
- [ ] Test 4: Authentication failures

### GUI Authentication Tests
- [ ] Test 5: Launch GUI
- [ ] Test 6: Access approval view
- [ ] Test 7: Failed authentication
- [ ] Test 8: Successful authentication

### GUI Request Display Tests
- [ ] Test 9: Empty state
- [ ] Test 10: Request list display
- [ ] Test 11: Select request
- [ ] Test 12: Approve request
- [ ] Test 13: Deny request
- [ ] Test 14: Refresh requests

### Other Views Tests
- [ ] Test 15: Dashboard
- [ ] Test 16: Reports
- [ ] Test 17: Content Filtering
- [ ] Test 18: Policy Config
- [ ] Test 19: Profile Editor

### Integration Tests
- [ ] Test 20: End-to-end approval workflow
- [ ] Test 21: Multiple profiles
- [ ] Test 22: Long-running session

### Edge Cases
- [ ] Test 23: Daemon not running
- [ ] Test 24: Network delays
- [ ] Test 25: Long request details
- [ ] Test 26: Special characters

## Sign-off

**Tester:** _________________  
**Date:** _________________  
**Overall Status:** Pass / Fail / Partial  
**Notes:**

---

## Quick Reference Commands

```bash
# Build everything
nix develop -c cargo build --workspace

# Run daemon
./result/bin/dots-family-daemon

# Run GUI
cargo run --package dots-family-gui

# Run CLI
cargo run --package dots-family-ctl -- approval list

# Check logs
journalctl -u dots-family-daemon -f

# Stop daemon
pkill dots-family-daemon
```
