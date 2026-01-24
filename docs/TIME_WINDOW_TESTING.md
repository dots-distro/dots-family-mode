# Time Window Enforcement Testing Guide

## Overview

This guide describes how to test the Time Window Enforcement feature for DOTS Family Mode. Time window enforcement restricts when child users can access the system based on configured time windows (e.g., 15:00-19:00 on weekdays).

## Feature Status

- Implementation: COMPLETE
- BDD Tests: 21/21 scenarios passing (100%)
- Integration: COMPLETE (profile activation hooked)
- VM Integration Test: Created but not yet run
- Manual Testing: PENDING

## Architecture

```
User Login
    │
    ├─> ProfileManager.set_active_profile()
    │   ├─> PolicyEngine.set_active_profile()
    │   └─> TimeWindowManager.set_active_profile() ✓
    │
    └─> TimeWindowEnforcementTask (runs every 60s)
        ├─> TimeWindowEnforcer.check_access()
        ├─> Warning notification (5 min before end)
        └─> EnforcementEngine.lock_session() (via loginctl)
```

## Testing Methods

### 1. BDD Tests (Unit/Integration)

Location: `tests/bdd/features/time_windows.feature`

Run all BDD tests:
```bash
cd tests/bdd
cargo test --test time_windows
```

Expected result: 21 scenarios passing, 176 steps passing

Coverage:
- Basic time window enforcement
- Weekend vs weekday windows
- Holiday handling
- Grace periods
- Midnight-spanning windows
- Overlapping windows
- Window extensions
- Timezone changes
- Manual time changes
- Per-user configurations
- Parent user exemption

### 2. VM Integration Test

Location: `tests/nix/time-window-test.nix`

Build and run the VM test:
```bash
# Run the automated test
nix build .#checks.x86_64-linux.time-window-enforcement

# Or launch interactive VM for manual testing
nix run .#nixosConfigurations.dots-family-test-vm.config.system.build.vm
```

Test configuration:
- child1: 06:00-08:00, 15:00-19:00 (weekdays), 08:00-21:00 (weekends)
- child2: 16:00-20:00 (weekdays), 09:00-22:00 (weekends)
- parent: No restrictions

### 3. Manual VM Testing

#### Step 1: Launch VM
```bash
nix run .#nixosConfigurations.dots-family-test-vm.config.system.build.vm
```

#### Step 2: Check Daemon Status
```bash
# Check if daemon is running
systemctl status dots-family-daemon

# Check daemon logs
journalctl -u dots-family-daemon -f
```

#### Step 3: Test Profile Activation
```bash
# Simulate user login by setting active profile
dots-family-ctl set-profile child1

# Check if profile was synced to time window manager
# Look for log message: "Profile <id> synced to time window manager"
journalctl -u dots-family-daemon -n 50 | grep "time window manager"
```

#### Step 4: Test Time Window Checking
```bash
# Check if current time allows access
dots-family-ctl check-time-window

# Expected output (within window):
# {"allowed": true, "timestamp": "2026-01-24T10:30:00+00:00"}

# Expected output (outside window):
# {"allowed": false, "timestamp": "2026-01-24T22:30:00+00:00"}
```

#### Step 5: Test Warning Notifications
To test warning notifications, you need to be within a time window and 5 minutes before it ends:

```bash
# Set system time to 5 minutes before window end
# (for child1 weekday window ending at 19:00)
sudo date -s "18:55"

# Wait 60 seconds for enforcement task to run
# Watch for notification on desktop

# Check logs
journalctl -u dots-family-daemon -n 20 | grep "warning"
```

#### Step 6: Test Session Locking
To test automatic session locking:

```bash
# Set system time to after window end
sudo date -s "19:01"

# Wait 60 seconds for enforcement task to run
# Session should lock automatically

# Check logs
journalctl -u dots-family-daemon -n 20 | grep "lock"

# Verify session is locked
loginctl list-sessions
```

#### Step 7: Test Manual Session Locking
```bash
# Manually trigger session lock via DBus
dots-family-ctl lock-session child1

# Check if session locked
loginctl list-sessions
# Look for "locked" status
```

#### Step 8: Test Next Window Query
```bash
# Get next available time window
dots-family-ctl get-next-window

# Expected output:
# {"start": "15:00", "end": "19:00", "available": true}
```

### 4. Real System Testing

For testing on a real NixOS system:

#### Installation
Add to `/etc/nixos/configuration.nix`:
```nix
{
  imports = [ ./path/to/dots-family-mode ];

  services.dots-family = {
    enable = true;
    parentUsers = [ "parent" ];
    childUsers = [ "child" ];
    
    profiles.child = {
      name = "Child";
      ageGroup = "8-12";
      timeWindows = [{
        start = "15:00";
        end = "19:00";
        days = [ "mon" "tue" "wed" "thu" "fri" ];
      }];
      weekendTimeWindows = [{
        start = "08:00";
        end = "21:00";
        days = [ "sat" "sun" ];
      }];
    };
  };
}
```

#### Testing Procedure

1. Rebuild system:
```bash
sudo nixos-rebuild switch
```

2. Check daemon status:
```bash
systemctl status dots-family-daemon
journalctl -u dots-family-daemon -f
```

3. Login as child user and verify:
   - Session starts successfully during allowed window
   - Warning notification appears 5 minutes before window ends
   - Session locks automatically when window expires
   - Can unlock with password but immediately re-locks

4. Login as parent user and verify:
   - No time window restrictions apply
   - Can login at any time

## Expected Behavior

### During Allowed Window
- User can login successfully
- No restrictions applied
- System functions normally

### 5 Minutes Before Window End
- Desktop notification appears: "Your computer time is ending soon. You have 5 minutes remaining."
- User can continue working
- Notification only appears once

### When Window Expires
- Session locks automatically (shows lock screen)
- Desktop notification: "Your computer time has ended. Please log out."
- User must enter password to unlock
- Session will re-lock immediately if still outside window

### Outside Window
- User cannot start new session (login blocked at display manager)
- If already logged in, session locks
- Notification shows next available window

### Parent Users
- No time window restrictions
- Can login at any time
- Not affected by enforcement

## Session Locking Implementation

The time window enforcement uses `loginctl lock-session` which is the systemd standard:

```bash
# Find user's session
loginctl list-sessions --no-legend | grep <username>

# Lock specific session
loginctl lock-session <session-id>
```

This works with all display managers (GDM, SDDM, LightDM, etc.) because:
- Display managers listen to systemd session lock signals
- Triggers the native lock screen (not a custom solution)
- User can unlock with their password
- Works across Wayland and X11

## Troubleshooting

### Profile Not Activated
Check logs for "synced to time window manager":
```bash
journalctl -u dots-family-daemon | grep "time window manager"
```

If not found, profile activation hook may not be working.

### Session Not Locking
Check if loginctl works:
```bash
# Get your session ID
loginctl list-sessions

# Try manual lock
loginctl lock-session <session-id>
```

If manual lock works but automatic doesn't:
- Check daemon logs for errors
- Verify enforcement task is running (logs every 60s)
- Check time window configuration

### Enforcement Task Not Running
Check daemon logs for periodic checks:
```bash
journalctl -u dots-family-daemon -f
# Should see activity every 60 seconds
```

### Time Zone Issues
Verify system timezone:
```bash
timedatectl status
```

Time windows use local time, so timezone must be configured correctly.

### Notifications Not Appearing
Check notification service:
```bash
# Verify notification-daemon is running
ps aux | grep notification

# Test manual notification
notify-send "Test" "This is a test notification"
```

## Test Scenarios

### Scenario 1: Basic Window Enforcement
1. Configure child with 15:00-19:00 weekday window
2. Set time to 16:00 on Monday
3. Login as child - should succeed
4. Wait for enforcement check (60s)
5. Set time to 20:00
6. Wait 60s - session should lock

### Scenario 2: Warning Notification
1. Set time to 18:55 (5 min before 19:00 end)
2. Login as child
3. Wait 60s - warning notification should appear
4. Wait 5 more minutes - session should lock at 19:00

### Scenario 3: Multiple Users
1. Configure child1: 15:00-18:00
2. Configure child2: 16:00-19:00
3. Set time to 15:30
4. Login as child1 - should succeed
5. Try login as child2 - should fail
6. Set time to 16:30
7. Both should be able to login

### Scenario 4: Weekend vs Weekday
1. Configure different windows for weekend/weekday
2. Set date to Monday, time to 10:00
3. Verify weekday window applies
4. Set date to Saturday, time to 10:00
5. Verify weekend window applies

### Scenario 5: Parent Exemption
1. Set time outside all windows
2. Try login as child - should fail
3. Try login as parent - should succeed

## Debug Commands

```bash
# Check daemon status
systemctl status dots-family-daemon

# View daemon logs (last 100 lines)
journalctl -u dots-family-daemon -n 100

# Follow daemon logs in real-time
journalctl -u dots-family-daemon -f

# Check active sessions
loginctl list-sessions

# Check time window via DBus
busctl call org.dots.FamilyDaemon /org/dots/FamilyDaemon \
  org.dots.FamilyDaemon check_time_window

# Get next window via DBus
busctl call org.dots.FamilyDaemon /org/dots/FamilyDaemon \
  org.dots.FamilyDaemon get_next_time_window

# Lock session via DBus
busctl call org.dots.FamilyDaemon /org/dots/FamilyDaemon \
  org.dots.FamilyDaemon lock_session s "child"
```

## Success Criteria

The time window enforcement feature is working correctly when:

1. BDD tests pass (21/21 scenarios)
2. Profile activation logs show sync to time window manager
3. Users can login during allowed windows
4. Warning notifications appear 5 minutes before window ends
5. Sessions lock automatically when windows expire
6. Parent users are not restricted
7. Per-user window configurations work correctly
8. Weekend/weekday windows apply correctly
9. Session locking works across different display managers
10. Users can unlock and use system during next window

## Implementation Details

### Files Modified for Integration
- `crates/dots-family-daemon/src/daemon.rs` - Store TimeWindowManager
- `crates/dots-family-daemon/src/dbus_impl.rs` - Hook profile activation

### Key Components
- `TimeWindowEnforcer` - Core logic (100% tested)
- `TimeWindowManager` - Session management wrapper
- `TimeWindowEnforcementTask` - Periodic background task (60s)
- `EnforcementEngine.lock_session()` - Session locking via loginctl

### Enforcement Interval
The enforcement task runs every 60 seconds. This means:
- There's up to 60 seconds of "grace" after a window ends
- Warning notifications may be delayed by up to 60 seconds
- This is intentional to avoid being too aggressive
- Can be adjusted by changing interval in daemon.rs:228

### State Management
The enforcement task tracks:
- `last_warning_sent`: Timestamp of last warning to prevent spam
- `session_locked`: Boolean to prevent repeated lock attempts

State is reset when profile changes or new window starts.

## Related Documentation

- BDD Workflow Guide: `docs/BDD_WORKFLOW_GUIDE.md`
- Architecture: `docs/ARCHITECTURE.md`
- NixOS Integration: `docs/NIXOS_INTEGRATION.md`

## Contact

For issues or questions about time window enforcement testing, check:
- GitHub Issues: https://github.com/anomalyco/opencode
- BDD Test Results: Run `cargo test --test time_windows` in tests/bdd/
