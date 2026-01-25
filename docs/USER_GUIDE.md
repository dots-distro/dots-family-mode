# DOTS Family Mode - User Guide

A comprehensive parental control system for NixOS families.

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Parent Dashboard](#parent-dashboard)
4. [Setting Up Child Profiles](#setting-up-child-profiles)
5. [Managing Time Limits](#managing-time-limits)
6. [Application Controls](#application-controls)
7. [Web Filtering](#web-filtering)
8. [Monitoring Activity](#monitoring-activity)
9. [Approval Requests](#approval-requests)
10. [Troubleshooting](#troubleshooting)

---

## Introduction

DOTS Family Mode provides comprehensive parental controls for NixOS systems, including:

- Screen time limits with age-appropriate defaults
- Application blocking and allowlisting
- Web content filtering with category-based blocking
- Real-time activity monitoring
- Approval request system for restricted content
- Time window restrictions (e.g., no internet after bedtime)
- Weekend bonus time
- Holiday schedules

### Key Features

- **Privacy-Focused**: All data stays on your local system
- **Transparent**: Open source and auditable
- **Flexible**: Customizable rules per child
- **Educational**: Helps children develop healthy digital habits
- **Reporting**: Detailed activity reports for parents

---

## Getting Started

### For Parents

After your system administrator has installed DOTS Family Mode, you can access the parental controls in two ways:

#### 1. Graphical Dashboard (Recommended)

Launch the DOTS Family Dashboard from your application menu:
- **GNOME**: Search for "DOTS Family Dashboard" in Activities
- **KDE Plasma**: Find it in System Settings → Security
- **XFCE**: Look in Settings → Parental Controls

#### 2. Command Line Interface

Open a terminal and use the `family` command:

```bash
# Check system status
family status

# Manage profiles
family profile list
family profile show alice

# View activity
family activity show alice --today
```

### For Children

When you log in, DOTS Family Mode will:
1. Show your current screen time remaining
2. Display any time restrictions
3. Monitor your activity in the background
4. Block restricted applications and websites

You'll see a friendly message when you open a terminal:
```
Welcome! Current family mode status:
  Screen Time Remaining: 1h 45m
  Time Window: Active until 8:00 PM
  
Have a great day!
```

---

## Parent Dashboard

### Overview Screen

The main dashboard shows:

- **Active Profiles**: All child accounts and their current status
- **Alerts**: Recent approval requests or policy violations
- **Daily Summary**: Screen time usage across all children
- **Quick Actions**: Common tasks like granting exceptions

### Navigation

- **Profiles Tab**: Manage child profiles and settings
- **Activity Tab**: View detailed activity logs
- **Approvals Tab**: Handle requests for blocked content
- **Reports Tab**: Generate weekly/monthly reports
- **Settings Tab**: System-wide configuration

---

## Setting Up Child Profiles

### Creating a Profile

1. Open the DOTS Family Dashboard
2. Click **"Add Profile"**
3. Fill in the details:
   - **Name**: Child's display name (e.g., "Alice")
   - **Username**: System username (must match NixOS user)
   - **Age Group**: Select from dropdown
     - **5-7**: Young children (very restrictive)
     - **8-12**: Elementary/Middle school (moderate)
     - **13-17**: Teenagers (balanced controls)
   - **Daily Screen Time**: Default based on age, adjustable

4. Click **"Create Profile"**

### Age Group Defaults

| Age Group | Daily Limit | Weekend Bonus | Web Filtering |
|-----------|-------------|---------------|---------------|
| 5-7       | 1 hour      | +30 minutes   | Strict        |
| 8-12      | 2 hours     | +30 minutes   | Moderate      |
| 13-17     | 3 hours     | +1 hour       | Light         |

These are starting points - adjust based on your family's needs.

### Editing a Profile

1. Select the profile from the list
2. Click **"Edit"**
3. Modify any settings:
   - Screen time limits
   - Time windows
   - Application rules
   - Web filtering level
4. Click **"Save Changes"**

---

## Managing Time Limits

### Daily Screen Time Limits

Screen time limits control total computer usage per day.

**Setting Limits:**
1. Select child's profile
2. Go to **"Time Limits"** section
3. Adjust the slider or enter hours:minutes
4. Click **"Apply"**

**How It Works:**
- Timer starts when child logs in
- Counts down during active use
- Pauses during idle time (5+ minutes)
- Resets at midnight

**Weekend Bonus:**
- Automatically adds extra time on Saturdays and Sundays
- Default: +30 minutes for younger children, +1 hour for teens
- Configurable per profile

### Time Windows

Restrict computer access to specific hours.

**Creating a Time Window:**
1. In profile settings, go to **"Time Windows"**
2. Click **"Add Window"**
3. Configure:
   - **Start Time**: When access begins (e.g., 3:00 PM)
   - **End Time**: When access ends (e.g., 8:00 PM)
   - **Days**: Select days of week
   - **Type**: School day, weekend, or custom

4. Click **"Save Window"**

**Example Configurations:**

**School Nights:**
```
Monday-Thursday: 4:00 PM - 8:00 PM
(After homework, before bedtime)
```

**Weekends:**
```
Saturday-Sunday: 9:00 AM - 9:00 PM
(More flexible schedule)
```

**Homework Time:**
```
Monday-Friday: 6:00 PM - 7:00 PM (BLOCKED)
(Dedicated study time)
```

### Holiday Schedules

Set special rules for school breaks.

1. Go to **"Holiday Schedule"**
2. Click **"Add Holiday Period"**
3. Configure:
   - **Name**: e.g., "Summer Break"
   - **Start Date**: First day of break
   - **End Date**: Last day of break
   - **Rules**: Relaxed limits, different time windows

---

## Application Controls

### Blocking Applications

**Method 1: Dashboard**
1. Select child's profile
2. Go to **"Applications"** tab
3. Click **"Block Application"**
4. Choose from detected applications or enter name
5. Click **"Block"**

**Method 2: Command Line**
```bash
# Block a specific application
family profile alice block-app firefox

# Block multiple apps
family profile alice block-app "discord,telegram,steam"
```

### Allowing Applications

By default, most applications are allowed. You can switch to an allowlist mode:

1. In profile settings, enable **"Allowlist Mode"**
2. Click **"Add Allowed Application"**
3. Select applications from the list
4. Only these apps will be permitted

**Recommended Allowlist for Young Children:**
- Web Browser (with filtering enabled)
- Educational apps (Khan Academy Kids, Scratch)
- Creative apps (Tux Paint, Blender)
- Office suite (for homework)

### Application Categories

Applications are grouped into categories for easier management:

- **Educational**: Learning apps (always allowed by default)
- **Games**: Entertainment software
- **Social Media**: Discord, social networks
- **Communication**: Email, messaging
- **Development**: Programming tools
- **Productivity**: Office suites, note-taking

You can block entire categories with one click.

---

## Web Filtering

### Filtering Levels

DOTS Family Mode provides three preset filtering levels:

#### Strict (Recommended for ages 5-10)
- Blocks adult content, violence, gambling
- Blocks social media and chat sites
- Allows educational sites
- Search engine safe mode enforced

#### Moderate (Recommended for ages 11-14)
- Blocks adult content and gambling
- Allows educational social media (supervised)
- Allows age-appropriate gaming sites
- YouTube restricted mode enabled

#### Light (Recommended for ages 15+)
- Blocks adult content only
- Allows most social media
- Minimal restrictions on general browsing

#### Custom
- Configure your own rules
- Choose specific categories to block
- Add site-specific exceptions

### Blocked Categories

- **Adult Content**: Explicit material
- **Gambling**: Casino, betting sites
- **Violence**: Graphic violent content
- **Weapons**: Firearms, dangerous items
- **Drugs**: Illegal substances
- **Hate Speech**: Discriminatory content

### Adding Site Exceptions

**Allow a Blocked Site:**
1. Child encounters blocked site
2. Click "Request Access"
3. Parent receives notification
4. Review request in dashboard
5. Grant temporary (24hr) or permanent exception

**Block an Allowed Site:**
1. In dashboard, go to **"Web Filtering"**
2. Click **"Block Site"**
3. Enter domain (e.g., "example.com")
4. Choose scope: This child only, or all children
5. Click **"Block"**

### Search Engine Safety

When web filtering is enabled:
- Google: SafeSearch forced on
- Bing: Strict filtering enabled
- DuckDuckGo: Moderate filtering
- YouTube: Restricted Mode

---

## Monitoring Activity

### Real-Time Status

See what your children are doing right now:

1. Dashboard home screen shows:
   - Currently active applications
   - Websites being visited
   - Screen time used today
   - Time remaining

### Activity Reports

**Daily Report:**
- Screen time used
- Top applications
- Websites visited
- Policy violations

**Weekly Report:**
- Usage trends (graph)
- Comparison between children
- Average screen time
- Recommendations

**Monthly Report:**
- Long-term trends
- Goal tracking
- Behavioral patterns

### Exporting Reports

1. Go to **"Reports"** tab
2. Select date range
3. Choose format: PDF, CSV, or print
4. Click **"Export"**

Reports include:
- Summary statistics
- Application usage breakdown
- Website categories visited
- Time window compliance
- Approval requests handled

---

## Approval Requests

Children can request access to blocked content. You'll receive notifications and can approve/deny requests.

### How It Works

1. **Child encounters blocked content**
   - Application is blocked
   - OR website is filtered
   - OR screen time limit reached

2. **Child clicks "Request Access"**
   - Enters reason (optional)
   - Request sent to parent

3. **Parent receives notification**
   - Desktop notification
   - Appears in dashboard "Approvals" tab

4. **Parent reviews request**
   - See what was blocked
   - View child's reason
   - Check usage history
   - Decide: Approve, Deny, or Grant Exception

### Approval Options

**Approve Once:**
- Access granted for this session only
- Expires when child logs out

**Approve for 24 Hours:**
- Temporary exception
- Automatically expires

**Add Permanent Exception:**
- Site/app permanently allowed
- Added to profile exceptions list

**Deny:**
- Request rejected
- Child notified

### Managing Approval Requests

**Command Line:**
```bash
# List pending requests
family approval list

# Approve a request
family approval approve <request-id>

# Deny a request
family approval deny <request-id> --reason "Not appropriate"
```

**Dashboard:**
1. Go to **"Approvals"** tab
2. See list of pending requests
3. Click on request for details
4. Choose action

---

## Troubleshooting

### Common Issues

#### Monitor Service Not Running

**Symptoms:**
- No activity tracking
- Screen time not counting down

**Solution:**
```bash
# Check service status
systemctl --user status dots-family-monitor.service

# Restart service
systemctl --user restart dots-family-monitor.service
```

#### Daemon Connection Failed

**Symptoms:**
- CLI commands return "Connection refused"
- Dashboard won't open

**Solution:**
```bash
# Check daemon status (run as parent user)
sudo systemctl status dots-family-daemon.service

# Check logs
sudo journalctl -u dots-family-daemon.service -n 50

# Restart daemon
sudo systemctl restart dots-family-daemon.service
```

#### Web Filtering Not Working

**Symptoms:**
- Blocked sites are accessible
- Filter appears inactive

**Solution:**
1. Check profile has web filtering enabled
2. Verify filtering level is not "None"
3. Check DNS settings (filter requires system DNS)
4. Restart web filter service:
```bash
sudo systemctl restart dots-family-filter.service
```

#### Screen Time Not Resetting

**Symptoms:**
- Timer doesn't reset at midnight
- Yesterday's time still counting

**Solution:**
This is usually a timezone issue:
```bash
# Check system timezone
timedatectl

# If wrong, set correct timezone
sudo timedatectl set-timezone America/New_York
```

#### Child Bypassing Controls

**Symptoms:**
- Screen time limits ignored
- Blocked apps accessible

**First Steps:**
1. Check child is logged in with correct username
2. Verify profile exists: `family profile list`
3. Check daemon is running
4. Review system logs for errors

**If Issue Persists:**
Contact your system administrator - there may be a configuration issue.

---

## Tips for Successful Parental Controls

### 1. Start With Communication

Before implementing controls:
- Explain why you're using parental controls
- Discuss appropriate online behavior
- Set clear expectations together
- Emphasize it's about safety, not punishment

### 2. Age-Appropriate Rules

- **Young children (5-7)**: Strict limits, supervised usage
- **Elementary (8-12)**: Moderate limits, teach responsibility
- **Teens (13-17)**: Lighter controls, build trust

### 3. Be Consistent

- Enforce rules fairly
- Don't make exceptions without reason
- Review and adjust rules together regularly

### 4. Monitor Without Hovering

- Check reports weekly, not hourly
- Look for patterns, not individual events
- Respect privacy while ensuring safety

### 5. Gradual Independence

As children demonstrate responsibility:
- Increase screen time limits
- Relax filtering levels
- Expand time windows
- Involve them in rule-making

### 6. Use Reports Constructively

- Discuss usage patterns with your child
- Celebrate positive behavior
- Address concerning trends early
- Set goals together (e.g., reduce social media time)

### 7. Lead by Example

Children mirror parents' technology use:
- Model healthy screen time habits
- Establish device-free family time
- Don't constantly check your phone
- Show them healthy tech-life balance

---

## Privacy and Data

### What Data Is Collected?

DOTS Family Mode collects:
- Application usage (names, duration)
- Website visits (domains only, not full URLs)
- Screen time duration
- Login/logout times
- Approval requests

### What Is NOT Collected?

- Keystrokes or passwords
- File contents
- Private messages
- Email contents
- Search queries (only domains)

### Where Is Data Stored?

All data is stored locally on your NixOS system:
- Database location: `/var/lib/dots-family/family.db`
- No data sent to external servers
- No cloud sync or analytics

### Data Retention

- Activity logs: 90 days (configurable)
- Approval requests: 30 days
- Reports: Generated on-demand, not stored
- Profiles: Kept until manually deleted

### Deleting Data

**Delete All Data for a Child:**
```bash
family profile alice delete --purge-data
```

**Delete Activity Logs Only:**
```bash
family activity clear alice --before 2024-01-01
```

---

## Getting Help

### Resources

- **User Guide**: This document
- **Admin Guide**: `/usr/share/doc/dots-family-mode/DEPLOYMENT.md`
- **Troubleshooting**: See section above
- **Issue Tracker**: https://github.com/dots-distro/dots-family-mode/issues

### Support

If you need help:
1. Check this guide's troubleshooting section
2. Review system logs: `journalctl -u dots-family-daemon.service`
3. Ask your system administrator
4. File an issue on GitHub with logs and error messages

### Contributing

DOTS Family Mode is open source! Contributions welcome:
- Report bugs
- Suggest features
- Submit pull requests
- Improve documentation

---

## Appendix: Quick Reference

### Common Commands

```bash
# Status and info
family status                          # System status
family profile list                    # List all profiles
family profile show alice              # Show profile details

# Activity monitoring
family activity show alice --today     # Today's activity
family activity show alice --week      # This week
family activity export alice report.pdf  # Export report

# Approvals
family approval list                   # Pending requests
family approval approve 123            # Approve request
family approval deny 123               # Deny request

# Time management
family time-left alice                 # Check remaining time
family time-add alice 30m              # Add 30 minutes (parent)
family time-reset alice                # Reset daily timer (admin)

# Application control
family profile alice block-app firefox    # Block app
family profile alice allow-app firefox    # Allow app

# Web filtering
family profile alice set-filter strict    # Change filter level
family profile alice allow-site github.com  # Allow site
family profile alice block-site example.com # Block site

# Exceptions
family exception grant alice "homework" 1h  # Grant 1-hour exception
family exception list alice                 # List active exceptions
```

### Keyboard Shortcuts (Dashboard)

- `Ctrl+P`: Switch profiles
- `Ctrl+A`: View approvals
- `Ctrl+R`: Refresh data
- `Ctrl+E`: Export report
- `Ctrl+Q`: Quit dashboard

---

## Glossary

**Age Group**: Predefined category (5-7, 8-12, 13-17) with default restrictions

**Allowlist**: List of explicitly permitted applications (everything else blocked)

**Blocklist**: List of explicitly blocked applications (everything else allowed)

**eBPF**: Kernel technology used for monitoring without performance impact

**Exception**: Temporary or permanent override of a rule

**Profile**: Collection of rules and settings for a specific child

**Screen Time**: Total duration of active computer use

**Time Window**: Specific hours when computer access is allowed/blocked

**Web Filtering**: Content-based blocking of inappropriate websites

---

*Last Updated: January 2026*  
*DOTS Family Mode Version: 0.1.0*
