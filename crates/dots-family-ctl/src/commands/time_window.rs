use anyhow::{anyhow, Result};
use dots_family_proto::daemon::FamilyDaemonProxy;
use zbus::Connection;

use crate::auth;

pub async fn add(
    profile: &str,
    weekday: bool,
    weekend: bool,
    holiday: bool,
    start: &str,
    end: &str,
) -> Result<()> {
    // Determine window type from flags
    let window_type = match (weekday, weekend, holiday) {
        (true, false, false) => "weekday",
        (false, true, false) => "weekend",
        (false, false, true) => "holiday",
        _ => {
            return Err(anyhow!(
                "Exactly one of --weekday, --weekend, or --holiday must be specified"
            ))
        }
    };

    // Validate time format
    validate_time_format(start)?;
    validate_time_format(end)?;

    // Validate start < end
    if start >= end {
        return Err(anyhow!("Start time must be before end time"));
    }

    let profile = profile.to_string();
    let start = start.to_string();
    let end = end.to_string();
    let window_type = window_type.to_string();

    // Require parent authentication
    auth::require_auth(|token| {
        Box::pin(async move {
            let conn = Connection::system().await?;
            let proxy = FamilyDaemonProxy::new(&conn).await?;

            let response =
                proxy.add_time_window(&profile, &window_type, &start, &end, &token).await?;

            // Parse response
            let result: serde_json::Value = serde_json::from_str(&response)?;

            if let Some(error) = result.get("error").and_then(|e| e.as_str()) {
                println!("Failed to add time window: {}", error);
            } else {
                println!(
                    "Successfully added {} time window {}–{} to profile '{}'",
                    window_type, start, end, profile
                );
            }

            Ok(())
        })
    })
    .await
}

pub async fn list(profile: &str) -> Result<()> {
    let profile = profile.to_string();

    // Require parent authentication
    auth::require_auth(|token| {
        Box::pin(async move {
            let conn = Connection::system().await?;
            let proxy = FamilyDaemonProxy::new(&conn).await?;

            let response = proxy.list_time_windows(&profile, &token).await?;

            // Parse response
            let result: serde_json::Value = serde_json::from_str(&response)?;

            if let Some(error) = result.get("error").and_then(|e| e.as_str()) {
                println!("Failed to list time windows: {}", error);
                return Ok(());
            }

            // Extract profile info
            let profile_name =
                result.get("profile_name").and_then(|n| n.as_str()).unwrap_or(&profile);

            println!("Time windows for profile '{}':", profile_name);

            // Display weekday windows
            if let Some(weekday) = result.get("weekday").and_then(|w| w.as_array()) {
                if !weekday.is_empty() {
                    println!("\n  Weekday:");
                    for window in weekday {
                        let start = window.get("start").and_then(|s| s.as_str()).unwrap_or("?");
                        let end = window.get("end").and_then(|e| e.as_str()).unwrap_or("?");
                        println!("    {}–{}", start, end);
                    }
                }
            }

            // Display weekend windows
            if let Some(weekend) = result.get("weekend").and_then(|w| w.as_array()) {
                if !weekend.is_empty() {
                    println!("\n  Weekend:");
                    for window in weekend {
                        let start = window.get("start").and_then(|s| s.as_str()).unwrap_or("?");
                        let end = window.get("end").and_then(|e| e.as_str()).unwrap_or("?");
                        println!("    {}–{}", start, end);
                    }
                }
            }

            // Display holiday windows
            if let Some(holiday) = result.get("holiday").and_then(|w| w.as_array()) {
                if !holiday.is_empty() {
                    println!("\n  Holiday:");
                    for window in holiday {
                        let start = window.get("start").and_then(|s| s.as_str()).unwrap_or("?");
                        let end = window.get("end").and_then(|e| e.as_str()).unwrap_or("?");
                        println!("    {}–{}", start, end);
                    }
                }
            }

            println!();
            Ok(())
        })
    })
    .await
}

pub async fn remove(
    profile: &str,
    weekday: bool,
    weekend: bool,
    holiday: bool,
    window: &str,
) -> Result<()> {
    // Determine window type from flags
    let window_type = match (weekday, weekend, holiday) {
        (true, false, false) => "weekday",
        (false, true, false) => "weekend",
        (false, false, true) => "holiday",
        _ => {
            return Err(anyhow!(
                "Exactly one of --weekday, --weekend, or --holiday must be specified"
            ))
        }
    };

    // Parse window (format: "HH:MM-HH:MM")
    let parts: Vec<&str> = window.split('-').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid window format '{}'. Expected HH:MM-HH:MM (e.g., 08:00-12:00)",
            window
        ));
    }

    let start = parts[0];
    let end = parts[1];

    // Validate time format
    validate_time_format(start)?;
    validate_time_format(end)?;

    let profile = profile.to_string();
    let start = start.to_string();
    let end = end.to_string();
    let window_type = window_type.to_string();

    // Require parent authentication
    auth::require_auth(|token| {
        Box::pin(async move {
            let conn = Connection::system().await?;
            let proxy = FamilyDaemonProxy::new(&conn).await?;

            let response =
                proxy.remove_time_window(&profile, &window_type, &start, &end, &token).await?;

            // Parse response
            let result: serde_json::Value = serde_json::from_str(&response)?;

            if let Some(error) = result.get("error").and_then(|e| e.as_str()) {
                println!("Failed to remove time window: {}", error);
            } else {
                println!(
                    "Successfully removed {} time window {}–{} from profile '{}'",
                    window_type, start, end, profile
                );
            }

            Ok(())
        })
    })
    .await
}

pub async fn clear(profile: &str, weekday: bool, weekend: bool, holiday: bool) -> Result<()> {
    // Determine window type from flags
    let window_type = match (weekday, weekend, holiday) {
        (true, false, false) => "weekday",
        (false, true, false) => "weekend",
        (false, false, true) => "holiday",
        _ => {
            return Err(anyhow!(
                "Exactly one of --weekday, --weekend, or --holiday must be specified"
            ))
        }
    };

    let profile = profile.to_string();
    let window_type = window_type.to_string();

    // Require parent authentication
    auth::require_auth(|token| {
        Box::pin(async move {
            let conn = Connection::system().await?;
            let proxy = FamilyDaemonProxy::new(&conn).await?;

            let response = proxy.clear_time_windows(&profile, &window_type, &token).await?;

            // Parse response
            let result: serde_json::Value = serde_json::from_str(&response)?;

            if let Some(error) = result.get("error").and_then(|e| e.as_str()) {
                println!("Failed to clear time windows: {}", error);
            } else {
                println!(
                    "Successfully cleared all {} time windows from profile '{}'",
                    window_type, profile
                );
            }

            Ok(())
        })
    })
    .await
}

/// Validate time format (HH:MM)
fn validate_time_format(time: &str) -> Result<()> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid time format '{}'. Expected HH:MM (e.g., 08:00, 15:30)", time));
    }

    let hours = parts[0]
        .parse::<u32>()
        .map_err(|_| anyhow!("Invalid hours '{}' in time '{}'", parts[0], time))?;
    let minutes = parts[1]
        .parse::<u32>()
        .map_err(|_| anyhow!("Invalid minutes '{}' in time '{}'", parts[1], time))?;

    if hours > 23 {
        return Err(anyhow!("Hours must be 0-23, got {}", hours));
    }
    if minutes > 59 {
        return Err(anyhow!("Minutes must be 0-59, got {}", minutes));
    }

    Ok(())
}
