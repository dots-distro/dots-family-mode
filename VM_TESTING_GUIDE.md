# DOTS Family Mode - VM Testing Guide

## VM Login Credentials

The VM has been configured with the following users:

### Users in VM
```
Parent User: parent / parent123
Child User:  child / child123
Root User:   root / root
```

## How to Proceed from Login Screen

### Option 1: Login via SSH (Recommended)

The VM is configured with SSH access:

```bash
# Connect as root
ssh -p 10022 root@localhost
# Password: root

# Or as parent user
ssh -p 10022 parent@localhost  
# Password: parent123
```

### Option 2: Use greetd with Kitty Terminal

If you can access a terminal from greetd:

1. At the greetd login screen, type: `parent`
2. Password: `parent123`
3. You'll get a graphical session with foot terminal

### Option 3: Use Virtual Machine Console

You can interact directly with the VM's virtual console.

## Running Tests via SSH

Once logged in via SSH, run:

```bash
# 1. Become root
sudo -i

# 2. Start the DOTS Family daemon
systemctl start dots-family-daemon.service

# 3. Check status
systemctl status dots-family-daemon.service

# 4. Check logs
journalctl -u dots-family-daemon.service -f

# 5. Test CLI
dots-family-ctl status
dots-family-ctl profile list

# 6. Test DBus communication
busctl call org.dots.FamilyDaemon /org/dots/FamilyDaemon \
  org.dots.FamilyDaemon GetVersion

# 7. Run the full integration test
# First, copy the test script to the VM:
scp -P 10022 scripts/vm_integration_test.sh root@localhost:/tmp/
ssh -p 10022 root@localhost "bash /tmp/vm_integration_test.sh"
```

## Quick Test Commands

### Check Service Status
```bash
systemctl status dots-family-daemon.service
```

### Start Service
```bash
systemctl start dots-family-daemon.service
```

### View Logs
```bash
journalctl -u dots-family-daemon.service --since "5 minutes ago" -f
```

### Test CLI
```bash
dots-family-ctl status
dots-family-ctl profile list
dots-family-ctl session list
```

### Test DBus
```bash
busctl list | grep dots
busctl call org.dots.FamilyDaemon /org/dots/FamilyDaemon org.dots.FamilyDaemon GetVersion
```

### Check Processes
```bash
ps aux | grep dots-family
```

### Check Network
```bash
ss -tunap
ip addr show
```

## Test Phases

### Phase 1: Service Validation
- [ ] Check systemd service files exist
- [ ] Start dots-family-daemon.service
- [ ] Verify service is active
- [ ] Check service logs

### Phase 2: DBus Communication
- [ ] Verify DBus is running
- [ ] Check org.dots.FamilyDaemon is registered
- [ ] Test DBus method calls

### Phase 3: CLI Testing
- [ ] Test dots-family-ctl help
- [ ] Test dots-family-ctl status
- [ ] Test dots-family-ctl profile commands

### Phase 4: Activity Monitoring
- [ ] Check dots-family-monitor service
- [ ] Verify window manager detection
- [ ] Test activity reporting

### Phase 5: Process & Network Monitoring
- [ ] Verify eBPF programs can load
- [ ] Check process monitoring
- [ ] Test network monitoring

## Expected Results

### Service Should Be Active
```
● dots-family-daemon.service - DOTS Family Mode Daemon
     Loaded: loaded (/etc/systemd/system/dots-family-daemon.service; enabled)
     Active: active (running)
```

### DBus Should Be Available
```
org.dots.FamilyDaemon          dots-family-daemon       dots-family-daemon     (active)
```

### CLI Should Respond
```
DOTS Family Mode Status
Status: active
Profiles: 1 configured
Monitoring: enabled
```

## Troubleshooting

### Service Won't Start
```bash
# Check logs for errors
journalctl -u dots-family-daemon.service -e

# Check if DBus is running
systemctl status dbus.service

# Verify configuration
cat /etc/dots-family/daemon.conf
```

### DBus Communication Failed
```bash
# Check DBus service
systemctl status dbus.service

# Verify service activation
busctl list | grep dots
```

### CLI Not Found
```bash
# Check if binaries are installed
which dots-family-ctl
dots-family-ctl --help

# Reinstall if missing
systemctl start dots-family-daemon.service
```

## Evidence Collection

All test results should be collected in:
- `/tmp/dots-family-test-evidence/`
- `/tmp/vm_test_evidence.md`

Copy evidence back to host:
```bash
scp -P 10022 root@localhost:/tmp/vm_test_evidence.md ./
```

## Next Steps

1. ✅ VM is booted and ready
2. ⏳ Login via SSH (recommended)
3. ⏳ Start and test daemon service
4. ⏳ Run full integration test
5. ⏳ Collect evidence
6. ⏳ Document results
