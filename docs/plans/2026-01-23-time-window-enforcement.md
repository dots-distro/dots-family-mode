# Time Window Enforcement - BDD Implementation Plan

**Date**: 2026-01-23  
**Status**: Planning  
**Method**: Behavior-Driven Development (BDD)

---

## Feature Overview

**Feature**: Time Window Enforcement  
**Parent Task**: `e642def8-7b2d-4828-bc42-c333d8d589d4`  
**Workflow**: `b36f3fb9-b2d9-49fd-94d8-85a3a29f6ddf`  
**Method**: Behavior-Driven Development (BDD)  
**Priority**: High (Core parental control feature)

## What is Time Window Enforcement?

From `docs/PARENTAL_CONTROLS.md` (lines 47-77):
- Restricts computer use to specific times of day
- Supports weekday/weekend/holiday windows  
- 5-minute warnings before window closes
- Grace period for saving work
- Parent override capability

### Example Configuration

```toml
[screen_time.windows]
enabled = true

# Weekday windows
weekday = [
  { start = "06:00", end = "08:00", label = "morning" },
  { start = "15:00", end = "19:00", label = "evening" }
]

# Weekend windows
weekend = [
  { start = "08:00", end = "21:00", label = "daytime" }
]

# Holiday windows (uses calendar)
holiday = [
  { start = "08:00", end = "21:00", label = "holiday" }
]
```

### Expected Behavior

- **Outside windows**: Cannot login or existing session locked
- **5-minute warning** before window closes
- **Grace period** for saving work (configurable)
- **Manual override** available to parent

---

## BDD Workflow (Red-Green-Refactor)

```
┌─────────────────────────────────────────────────────────────┐
│  BDD Cycle for Time Window Enforcement                      │
└─────────────────────────────────────────────────────────────┘

1. 📝 Write Gherkin Specs (75aef678)
   └──> Define behavior in plain language
        
2. 🔧 Set up Cucumber Framework (c3c4d6c8)
   └──> Configure cucumber-rust, add to project
        ↓ depends_on (1)
        
3. 🔴 Write Step Definitions - RED (0390c634)
   └──> Implement test steps, expect failures
        ↓ depends_on (2)
        
4. 🟢 Implement Feature Logic - GREEN (c2aa22fd)
   └──> Write minimal code to pass tests
        ↓ depends_on (3)
        
5. ♻️  Refactor - Keep GREEN (ce02255a)
   └──> Clean up code, maintain passing tests
        ↓ depends_on (4)
        
6. 🧪 VM Integration Tests (920724a3)
   └──> Add to full-deployment-test.nix
        ↓ depends_on (5)
```

---

## Task Breakdown

| # | Task | ID | Status | Depends On |
|---|------|-----|--------|------------|
| 1 | Write Gherkin feature specs | `75aef678-2740-4ef6-a6b5-22e73358a993` | Todo | - |
| 2 | Set up Cucumber/BDD framework | `c3c4d6c8-8ab0-48de-92dd-fd21818f29df` | Todo | Task 1 |
| 3 | Write step definitions (RED) | `0390c634-df54-489e-bdb5-6624e656b9ee` | Todo | Task 2 |
| 4 | Implement feature logic (GREEN) | `c2aa22fd-8bc9-4809-b252-bde125b875d2` | Todo | Task 3 |
| 5 | Refactor implementation | `ce02255a-4e29-4b3e-92f6-ac472546d209` | Todo | Task 4 |
| 6 | VM integration tests | `920724a3-a28e-4892-84df-d95b53293f00` | Todo | Task 5 |

---

## Example Gherkin Specifications

### Task 1: Write Feature Specs

Location: `tests/features/time_window_enforcement.feature`

```gherkin
Feature: Time Window Enforcement
  As a parent
  I want to restrict computer access to specific times
  So that my child maintains healthy screen time habits

  Background:
    Given a child profile "alice" with age group "8-12"
    And the current time is "2026-01-24 09:00:00"

  Scenario: Allow access during permitted time window
    Given a weekday time window from "06:00" to "08:00"
    And a weekday time window from "15:00" to "19:00"
    When alice attempts to login at "07:30"
    Then the login should succeed
    And the session should be active

  Scenario: Block access outside permitted time window
    Given a weekday time window from "06:00" to "08:00"
    And a weekday time window from "15:00" to "19:00"
    When alice attempts to login at "10:00"
    Then the login should be blocked
    And a message "Outside permitted time window" should be displayed

  Scenario: Warning before window closes
    Given a weekday time window from "15:00" to "19:00"
    And alice is logged in at "18:54"
    When the time reaches "18:55"
    Then a 5-minute warning should be displayed
    And the warning should say "Time window closes in 5 minutes"

  Scenario: Session locks when window closes
    Given a weekday time window from "15:00" to "19:00"
    And alice is logged in at "18:58"
    When the time reaches "19:00"
    Then the session should lock
    And unsaved work should have been prompted to save

  Scenario: Parent override allows access
    Given a weekday time window from "15:00" to "19:00"
    And alice is blocked from logging in at "10:00"
    When parent enters override password
    Then alice should be granted temporary access
    And the override should be logged

  Scenario: Weekend windows differ from weekday
    Given a weekend time window from "08:00" to "21:00"
    And today is Saturday
    When alice attempts to login at "09:00"
    Then the login should succeed
    When alice attempts to login at "22:00"
    Then the login should be blocked

  Scenario: Grace period for saving work
    Given a weekday time window from "15:00" to "19:00"
    And alice is logged in at "18:59"
    And grace period is set to "2 minutes"
    When the time reaches "19:00"
    Then a save prompt should be displayed
    And the session should remain active for 2 minutes
    When the time reaches "19:02"
    Then the session should force lock
```

---

## Technical Implementation Details

### Task 2: Set up Cucumber Framework

**Dependencies to add to `Cargo.toml`**:
```toml
[dev-dependencies]
cucumber = "0.20"
tokio = { version = "1.0", features = ["full", "test-util"] }
```

**Directory structure**:
```
tests/
├── features/
│   ├── time_window_enforcement.feature
│   └── support/
│       └── world.rs
└── steps/
    ├── mod.rs
    └── time_window_steps.rs
```

**Test runner** (`tests/cucumber.rs`):
```rust
use cucumber::World;

#[tokio::main]
async fn main() {
    TimeWindowWorld::run("tests/features")
        .await;
}
```

### Task 3: Write Step Definitions (RED Phase)

Location: `tests/steps/time_window_steps.rs`

```rust
use cucumber::{given, when, then, World};
use chrono::{DateTime, Utc, NaiveTime};

#[derive(Debug, World)]
pub struct TimeWindowWorld {
    profile: Option<Profile>,
    current_time: DateTime<Utc>,
    time_windows: Vec<TimeWindow>,
    login_result: Option<LoginResult>,
    session_state: Option<SessionState>,
}

#[given(expr = "a child profile {string} with age group {string}")]
async fn create_profile(world: &mut TimeWindowWorld, name: String, age_group: String) {
    world.profile = Some(Profile::new(name, age_group));
}

#[given(expr = "the current time is {string}")]
async fn set_current_time(world: &mut TimeWindowWorld, time: String) {
    world.current_time = DateTime::parse_from_rfc3339(&time)
        .unwrap()
        .with_timezone(&Utc);
}

#[given(expr = "a weekday time window from {string} to {string}")]
async fn add_weekday_window(world: &mut TimeWindowWorld, start: String, end: String) {
    let window = TimeWindow {
        start: NaiveTime::parse_from_str(&start, "%H:%M").unwrap(),
        end: NaiveTime::parse_from_str(&end, "%H:%M").unwrap(),
        days: vec![DayOfWeek::Monday, DayOfWeek::Tuesday, /* ... */],
    };
    world.time_windows.push(window);
}

#[when(expr = "{} attempts to login at {string}")]
async fn attempt_login(world: &mut TimeWindowWorld, profile: String, time: String) {
    let login_time = NaiveTime::parse_from_str(&time, "%H:%M").unwrap();
    // This will fail initially (RED) - implement in Task 4
    world.login_result = Some(
        TimeWindowEnforcer::check_login(
            &world.time_windows,
            login_time,
            &world.profile.as_ref().unwrap()
        ).await
    );
}

#[then(expr = "the login should succeed")]
async fn login_succeeds(world: &mut TimeWindowWorld) {
    assert!(world.login_result.as_ref().unwrap().is_allowed());
}

#[then(expr = "the login should be blocked")]
async fn login_blocked(world: &mut TimeWindowWorld) {
    assert!(!world.login_result.as_ref().unwrap().is_allowed());
}
```

### Task 4: Implement Feature Logic (GREEN Phase)

Location: `crates/dots-family-daemon/src/time_window_enforcer.rs`

```rust
use chrono::{DateTime, NaiveTime, Datelike, Utc};
use crate::profile::Profile;

pub struct TimeWindowEnforcer {
    windows: Vec<TimeWindow>,
}

impl TimeWindowEnforcer {
    pub async fn check_login(
        windows: &[TimeWindow],
        current_time: NaiveTime,
        profile: &Profile,
    ) -> LoginResult {
        let day_of_week = Utc::now().weekday();
        
        // Check if current time falls within any allowed window
        for window in windows {
            if window.days.contains(&day_of_week) {
                if current_time >= window.start && current_time < window.end {
                    return LoginResult::Allowed;
                }
            }
        }
        
        LoginResult::Blocked {
            reason: "Outside permitted time window".to_string(),
        }
    }
    
    pub async fn start_monitoring(&self) {
        // Start background task to check for window closures
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            self.check_window_expiration().await;
        }
    }
    
    async fn check_window_expiration(&self) {
        let now = Utc::now();
        let current_time = now.time();
        
        // Check if we're 5 minutes before window end
        for window in &self.windows {
            let warning_time = window.end - Duration::from_secs(5 * 60);
            if current_time >= warning_time && current_time < window.end {
                self.send_warning(window).await;
            }
            
            // Check if window has closed
            if current_time >= window.end {
                self.lock_sessions(window).await;
            }
        }
    }
    
    async fn send_warning(&self, window: &TimeWindow) {
        // Send DBus notification
        let notification = Notification {
            title: "Time Window Closing".to_string(),
            body: format!("Time window closes in 5 minutes at {}", window.end),
            urgency: NotificationUrgency::Critical,
        };
        
        self.dbus_conn.send_notification(notification).await;
    }
    
    async fn lock_sessions(&self, window: &TimeWindow) {
        // Lock all active sessions for this profile
        let sessions = self.session_manager.get_active_sessions().await;
        
        for session in sessions {
            if self.should_lock_session(&session, window).await {
                session.lock().await;
            }
        }
    }
}
```

### Task 5: Refactor Implementation

**Refactoring goals**:
- Extract time calculation logic into separate module
- Add comprehensive error handling
- Improve logging and telemetry
- Add configuration validation
- Document public API

**Example refactoring**:
```rust
// Before (Task 4)
if current_time >= window.start && current_time < window.end {
    return LoginResult::Allowed;
}

// After (Task 5)
if window.contains(current_time, day_of_week) {
    return LoginResult::Allowed {
        window: window.clone(),
        expires_at: window.end,
    };
}
```

### Task 6: VM Integration Tests

Location: `tests/nix/full-deployment-test.nix`

Add to test script:
```python
with subtest("Time windows are enforced"):
    # Create profile with time window
    machine.succeed("""
        dots-family-ctl profile create timechild 8-12
        dots-family-ctl profile config timechild --add-window weekday 09:00 17:00
    """)
    
    # Test access during allowed time
    machine.succeed("timedatectl set-time '2026-01-24 10:00:00'")
    result = machine.succeed("dots-family-ctl session check timechild")
    assert "allowed" in result.lower()
    
    # Test access outside allowed time
    machine.succeed("timedatectl set-time '2026-01-24 20:00:00'")
    result = machine.succeed("dots-family-ctl session check timechild")
    assert "blocked" in result.lower()
    
    # Test warning notification
    machine.succeed("timedatectl set-time '2026-01-24 16:55:00'")
    # Wait for warning notification
    machine.wait_until_succeeds("journalctl -u dots-family-daemon | grep 'Time window closes'")

with subtest("Parent override works"):
    machine.succeed("timedatectl set-time '2026-01-24 20:00:00'")
    machine.succeed("dots-family-ctl override grant timechild 30m --parent-password test")
    result = machine.succeed("dots-family-ctl session check timechild")
    assert "override active" in result.lower()
```

---

## Engram Tracking

All tasks are tracked in engram with proper relationships:

### View task status:
```bash
engram task list | grep -E "Time Window|Gherkin|Cucumber"
```

### View relationships:
```bash
engram relationship list | grep e642def8
```

### Start working on a task:
```bash
# Mark task as active (once previous task is complete)
engram task update <TASK_ID> --status todo
```

### Complete a task:
```bash
engram task update <TASK_ID> --status done --outcome "Description of what was completed"
```

---

## Success Criteria

- [ ] Gherkin specs written and reviewed (Task 1)
- [ ] Cucumber framework integrated (Task 2)
- [ ] All step definitions implemented (Task 3)
- [ ] Feature logic passes all BDD tests (Task 4)
- [ ] Code is refactored and clean (Task 5)
- [ ] VM integration tests pass (Task 6)
- [ ] Documentation updated
- [ ] All commits reference engram task IDs
- [ ] No regression in existing tests

---

## Getting Started

### Step 1: Start Task 1 (Write Gherkin Specs)

```bash
# View task details
engram task list | grep "Gherkin"

# Create feature file directory
mkdir -p tests/features

# Create the feature file
touch tests/features/time_window_enforcement.feature

# Start editing with the example specs from this document
```

### Step 2: Review and Commit

```bash
# Add the specs
git add tests/features/time_window_enforcement.feature

# Commit with task reference
git commit -m "feat: add Gherkin specs for time window enforcement [75aef678-2740-4ef6-a6b5-22e73358a993]

- Define behavior scenarios for time windows
- Include weekday/weekend/holiday cases
- Add warning and grace period scenarios
- Document parent override behavior"

# Mark task as complete
engram task update 75aef678-2740-4ef6-a6b5-22e73358a993 --status done --outcome "Gherkin feature specs created with comprehensive scenarios"
```

### Step 3: Move to Task 2

Once Task 1 is complete, Task 2 (Set up Cucumber Framework) becomes available due to the dependency chain.

---

## References

- **Specification**: `docs/PARENTAL_CONTROLS.md` lines 47-77
- **Cucumber Rust**: https://github.com/cucumber-rs/cucumber
- **BDD Best Practices**: Write scenarios from user perspective, focus on behavior not implementation
- **Commit Convention**: All commits must reference engram task IDs

---

**Status**: Ready to begin Task 1  
**Next Action**: Create Gherkin feature specifications  
**Owner**: Development team using BDD methodology
