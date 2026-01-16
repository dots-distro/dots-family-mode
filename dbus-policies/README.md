# DOTS Family Mode - DBus Security Policies

This directory contains DBus policy configuration files for secure system-wide integration of the DOTS Family Mode parental control system.

## Files

### Policy Files

- **`org.dots.FamilyDaemon.conf`** - System bus policy for the main daemon service
  - Controls access to the `org.dots.FamilyDaemon` interface
  - Implements role-based access control (root, parents, family members, monitor)
  - Enforces method-level permissions for security

- **`org.dots.FamilyMonitor.conf`** - Session bus policy for monitor services  
  - Controls access to the `org.dots.FamilyMonitor` interface
  - Allows per-user monitor instances on session bus
  - Enables secure activity monitoring and reporting

### Installation Script

- **`install-dbus-policies.sh`** - Automated installation script
  - Creates required system users and groups
  - Installs policy files to correct system directories
  - Validates installation and reloads DBus configuration
  - Must be run as root (`sudo ./install-dbus-policies.sh`)

## Security Model

### User Groups

1. **`dots-parents`** - Parent/administrator users
   - Full access to all daemon methods including administrative functions
   - Can authenticate, create profiles, and manage system settings
   - Receives all signals and notifications

2. **`dots-family`** - Child/family users  
   - Read-only access to most daemon methods
   - Can check permissions and request parent approval
   - Receives policy updates and time warnings
   - Cannot modify system configuration

3. **`dots-monitor`** - System user for monitor processes
   - Can report activity to daemon
   - Can send heartbeat signals  
   - Can check application permissions
   - Cannot access administrative functions

### Permissions Matrix

| Method | Root | Parents | Family | Monitor |
|--------|------|---------|--------|---------|
| `get_active_profile` | ✓ | ✓ | ✓ | - |
| `check_application_allowed` | ✓ | ✓ | ✓ | ✓ |
| `get_remaining_time` | ✓ | ✓ | ✓ | - |
| `report_activity` | ✓ | - | - | ✓ |
| `send_heartbeat` | ✓ | - | - | ✓ |
| `authenticate_parent` | ✓ | ✓ | - | - |
| `create_profile` | ✓ | ✓ | - | - |
| `set_active_profile` | ✓ | ✓ | - | - |
| `request_parent_permission` | ✓ | ✓ | ✓ | - |
| `request_command_approval` | ✓ | ✓ | ✓ | - |

### Signal Permissions

- **Policy Updates**: All authenticated users receive `policy_updated` signals
- **Time Warnings**: Family users receive `time_limit_warning` signals  
- **Tamper Detection**: Parents receive `tamper_detected` signals

## Installation

### Prerequisites

- systemd-based Linux distribution
- DBus system and session buses running
- Root access for installation

### Quick Install

```bash
# Install policies and create system users/groups
sudo ./install-dbus-policies.sh

# Add parent users to administrative group
sudo usermod -a -G dots-parents <parent-username>

# Add child users to family group  
sudo usermod -a -G dots-family <child-username>
```

### Manual Installation

```bash
# Copy policy files
sudo cp org.dots.FamilyDaemon.conf /etc/dbus-1/system.d/
sudo cp org.dots.FamilyMonitor.conf /etc/dbus-1/session.d/

# Create system groups
sudo groupadd -r dots-family
sudo groupadd -r dots-parents

# Create monitor user
sudo useradd -r -s /bin/false dots-monitor

# Reload DBus configuration
sudo systemctl reload dbus.service
```

## Verification

### Test DBus Policy Installation

```bash
# Check policy files are installed
ls -l /etc/dbus-1/system.d/org.dots.FamilyDaemon.conf
ls -l /etc/dbus-1/session.d/org.dots.FamilyMonitor.conf

# Verify groups exist
getent group dots-family dots-parents

# Verify monitor user  
id dots-monitor

# Test DBus introspection (after daemon is running)
dbus-send --system --print-reply \
  --dest=org.dots.FamilyDaemon \
  /org/dots/FamilyDaemon \
  org.freedesktop.DBus.Introspectable.Introspect
```

### Common Issues

1. **"Permission denied" errors**
   - Verify user is in correct group (`groups <username>`)
   - Check policy file permissions (`ls -l /etc/dbus-1/`)
   - Ensure DBus was reloaded after policy installation

2. **"Service not found" errors**  
   - Verify systemd service is running (`systemctl status dots-family-daemon`)
   - Check service activation file installation
   - Verify daemon binary is in correct path

3. **Group membership not effective**
   - User must log out and back in for group membership to take effect
   - Or use `newgrp dots-family` in current session

## Development Testing

For testing without full system installation:

```bash
# Test policy syntax
dbus-validate-policy org.dots.FamilyDaemon.conf
dbus-validate-policy org.dots.FamilyMonitor.conf

# Run daemon with custom bus address for testing
export DBUS_SYSTEM_BUS_ADDRESS="unix:path=/tmp/dbus-test"
```

## Integration with NixOS

These policies will be integrated into the NixOS module for declarative system configuration. See `../nixos-modules/` for module implementation.

## Security Considerations

- Policies follow principle of least privilege
- Monitor processes run as unprivileged system user
- Administrative functions require parent group membership
- Default deny policy blocks unauthorized access
- All sensitive operations require explicit permission grants

## Related Files

- `../systemd/` - systemd service configurations that reference these policies
- `../nixos-modules/` - NixOS module that automates policy installation  
- `../crates/dots-family-proto/` - DBus interface definitions these policies secure