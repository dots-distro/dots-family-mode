Feature: Time Window Enforcement
  As a parent
  I want to restrict computer access to specific time windows
  So that my children use the computer only during appropriate hours

  Background:
    Given the time window feature is enabled
    And the system time zone is configured correctly

  # Basic Time Window Rules

  Scenario: Child can login during weekday morning window
    Given the current day is "Monday"
    And the current time is "07:00"
    And weekday windows are configured as:
      | start | end   | label   |
      | 06:00 | 08:00 | morning |
      | 15:00 | 19:00 | evening |
    When a child user attempts to login
    Then the login should succeed
    And the session should be active

  Scenario: Child cannot login outside weekday windows
    Given the current day is "Tuesday"
    And the current time is "10:00"
    And weekday windows are configured as:
      | start | end   | label   |
      | 06:00 | 08:00 | morning |
      | 15:00 | 19:00 | evening |
    When a child user attempts to login
    Then the login should be denied
    And a message should explain "Computer access is restricted to: 06:00-08:00, 15:00-19:00"
    And the next available window should be shown as "15:00"

  Scenario: Child can login during weekend window
    Given the current day is "Saturday"
    And the current time is "14:00"
    And weekend windows are configured as:
      | start | end   | label   |
      | 08:00 | 21:00 | daytime |
    When a child user attempts to login
    Then the login should succeed
    And the session should be active

  Scenario: Child cannot login before weekend window starts
    Given the current day is "Sunday"
    And the current time is "07:30"
    And weekend windows are configured as:
      | start | end   | label   |
      | 08:00 | 21:00 | daytime |
    When a child user attempts to login
    Then the login should be denied
    And a message should explain "Computer access starts at 08:00"

  Scenario: Holiday window overrides weekday schedule
    Given the current day is "Monday" marked as holiday
    And the current time is "10:00"
    And weekday windows are configured as:
      | start | end   | label   |
      | 06:00 | 08:00 | morning |
      | 15:00 | 19:00 | evening |
    And holiday windows are configured as:
      | start | end   | label   |
      | 08:00 | 21:00 | holiday |
    When a child user attempts to login
    Then the login should succeed
    And the session should be active

  # Window Transition Scenarios

  Scenario: Session locked when time window ends
    Given a child user is logged in
    And the current day is "Wednesday"
    And the current time is "08:00"
    And weekday windows are configured as:
      | start | end   | label   |
      | 06:00 | 08:00 | morning |
      | 15:00 | 19:00 | evening |
    When the time reaches "08:00"
    Then the session should be locked immediately
    And all user processes should be suspended
    And a notification should display "Time window has ended"

  Scenario: 5-minute warning before window closes
    Given a child user is logged in
    And the current day is "Thursday"
    And the current time is "18:54"
    And weekday windows are configured as:
      | start | end   | label   |
      | 06:00 | 08:00 | morning |
      | 15:00 | 19:00 | evening |
    When the time reaches "18:55"
    Then a warning notification should be displayed
    And the notification should say "5 minutes remaining in this window"
    And the notification should persist until window ends

  Scenario: Grace period allows work to be saved
    Given a child user is logged in with unsaved work
    And the current day is "Friday"
    And the current time is "18:59"
    And weekday windows are configured as:
      | start | end   | label   |
      | 06:00 | 08:00 | morning |
      | 15:00 | 19:00 | evening |
    And grace period is configured as "2 minutes"
    When the time reaches "19:00"
    Then a grace period countdown should start
    And the user should have 2 minutes to save work
    And the session should lock at "19:02"

  Scenario: Multiple windows in same day
    Given the current day is "Monday"
    And weekday windows are configured as:
      | start | end   | label   |
      | 06:00 | 08:00 | morning |
      | 15:00 | 19:00 | evening |
    When the current time is "07:00"
    Then login should succeed
    When the current time is "09:00"
    Then login should be denied
    When the current time is "16:00"
    Then login should succeed

  # Parent Override Scenarios

  Scenario: Parent can manually override time restriction
    Given a child user is locked out due to time window
    And the current day is "Tuesday"
    And the current time is "10:00"
    And weekday windows are configured as:
      | start | end   | label   |
      | 06:00 | 08:00 | morning |
      | 15:00 | 19:00 | evening |
    When a parent issues an override command
    And specifies duration "30 minutes"
    Then the child user should be able to login
    And the override should expire after 30 minutes
    And the session should lock when override expires

  Scenario: Parent override logged for audit
    Given a parent issues a time window override
    When the override is activated
    Then an audit log entry should be created
    And the entry should include:
      | field         | value                    |
      | event_type    | time_window_override     |
      | parent_id     | parent user ID           |
      | child_id      | child user ID            |
      | timestamp     | current timestamp        |
      | duration      | override duration        |
      | reason        | optional parent comment  |

  Scenario: Parent can extend active session
    Given a child user is logged in
    And the current time is "18:55"
    And the window will close at "19:00"
    When a parent extends the session by "30 minutes"
    Then the window end time should be extended to "19:30"
    And the child session should remain active
    And a notification should inform the child of the extension

  # Edge Cases and Boundary Conditions

  Scenario: Window spans midnight
    Given weekend windows are configured as:
      | start | end   | label     |
      | 08:00 | 23:59 | daytime   |
    And the current day is "Saturday"
    And the current time is "23:58"
    When a child user is logged in
    Then the session should remain active until "23:59"
    And at "00:00" the session should lock

  Scenario: Overlapping windows use earliest start and latest end
    Given weekday windows are configured as:
      | start | end   | label   |
      | 06:00 | 10:00 | morning |
      | 08:00 | 12:00 | midday  |
    And the current day is "Monday"
    When the current time is "09:00"
    Then login should succeed
    And the effective window should be "06:00 to 12:00"

  Scenario: Empty window configuration denies all access
    Given time windows are enabled
    But no windows are configured for the current day type
    And the current day is "Monday"
    When a child user attempts to login
    Then the login should be denied
    And a message should explain "No time windows configured for today"

  Scenario: Disabled time windows allow unrestricted access
    Given the time window feature is disabled
    When a child user attempts to login at any time
    Then the login should always succeed

  # System Clock Changes

  Scenario: Time zone change handled correctly
    Given a child user is logged in
    And the current time is "17:00 UTC"
    And weekday windows are configured as:
      | start | end   | label   |
      | 15:00 | 19:00 | evening |
    When the system time zone changes from "UTC" to "UTC+2"
    Then the local time should be recalculated to "19:00"
    And the window enforcement should use local time
    And the session should lock if outside window

  Scenario: Manual time change detected
    Given a child user is logged in
    And the current time is "16:00"
    When the system time is manually changed to "10:00"
    Then the time change should be detected
    And window enforcement should re-evaluate immediately
    And if outside window, session should lock
    And an audit log entry should record the time change

  # Multi-user Scenarios

  Scenario: Different users have different window configurations
    Given user "child1" has weekday windows:
      | start | end   | label   |
      | 15:00 | 18:00 | evening |
    And user "child2" has weekday windows:
      | start | end   | label   |
      | 16:00 | 19:00 | evening |
    And the current day is "Monday"
    And the current time is "15:30"
    When "child1" attempts to login
    Then login should succeed
    When "child2" attempts to login
    Then login should be denied

  Scenario: Parent user not subject to time windows
    Given the time window feature is enabled
    And the current time is outside all configured windows
    When a parent user attempts to login
    Then the login should succeed
    And no time window restrictions should apply
