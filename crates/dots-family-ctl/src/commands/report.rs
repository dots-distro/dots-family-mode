use anyhow::Result;
use dots_family_proto::daemon::FamilyDaemonProxy;
use zbus::Connection;

use crate::auth;

/// Get a daily activity report for a profile
pub async fn daily(profile: &str, date: Option<&str>) -> Result<()> {
    let profile = profile.to_string();
    let date = if let Some(d) = date {
        d.to_string()
    } else {
        // Default to today - use system command
        let output = std::process::Command::new("date").arg("+%Y-%m-%d").output()?;
        String::from_utf8(output.stdout)?.trim().to_string()
    };

    // Require parent authentication
    auth::require_auth(|_token| {
        Box::pin(async move {
            let conn = Connection::system().await?;
            let proxy = FamilyDaemonProxy::new(&conn).await?;

            let response = proxy.get_daily_report(&profile, &date).await?;

            // Parse response
            let result: serde_json::Value = serde_json::from_str(&response)?;

            if let Some(error) = result.get("error").and_then(|e| e.as_str()) {
                println!("Failed to get daily report: {}", error);
                return Ok(());
            }

            // Display the report
            println!("\n📊 Daily Activity Report for {}", date);
            println!("═══════════════════════════════════════════════\n");

            if let Some(screen_time) = result.get("screen_time_minutes").and_then(|s| s.as_u64()) {
                let hours = screen_time / 60;
                let minutes = screen_time % 60;
                println!("⏱️  Total Screen Time: {}h {}m", hours, minutes);
            }

            if let Some(top_activity) = result.get("top_activity").and_then(|t| t.as_str()) {
                println!("🏆 Top Activity: {}", top_activity);
            }

            if let Some(top_category) = result.get("top_category").and_then(|t| t.as_str()) {
                println!("📁 Top Category: {}", top_category);
            }

            if let Some(violations) = result.get("violations").and_then(|v| v.as_u64()) {
                println!("⚠️  Policy Violations: {}", violations);
            }

            if let Some(blocked) = result.get("blocked_attempts").and_then(|b| b.as_u64()) {
                println!("🚫 Blocked Attempts: {}", blocked);
            }

            // Display app usage breakdown
            if let Some(apps) = result.get("apps_used").and_then(|a| a.as_array()) {
                if !apps.is_empty() {
                    println!("\n📱 Application Usage:");
                    println!("─────────────────────────────────────────────");
                    for (i, app) in apps.iter().enumerate().take(10) {
                        let app_name =
                            app.get("app_name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                        let category =
                            app.get("category").and_then(|c| c.as_str()).unwrap_or("Unknown");
                        let duration =
                            app.get("duration_minutes").and_then(|d| d.as_u64()).unwrap_or(0);
                        let percentage =
                            app.get("percentage").and_then(|p| p.as_f64()).unwrap_or(0.0);

                        let hours = duration / 60;
                        let minutes = duration % 60;

                        println!("  {}. {} ({})", i + 1, app_name, category);
                        println!("     {}h {}m ({:.1}%)", hours, minutes, percentage);
                    }
                }
            }

            println!();
            Ok(())
        })
    })
    .await
}

/// Get a weekly activity report for a profile
pub async fn weekly(profile: &str, week_start: Option<&str>) -> Result<()> {
    let profile = profile.to_string();
    let week_start = if let Some(w) = week_start {
        w.to_string()
    } else {
        // Default to start of current week (Monday) - use system command
        let output = std::process::Command::new("date")
            .arg("-d")
            .arg("last monday")
            .arg("+%Y-%m-%d")
            .output()?;
        String::from_utf8(output.stdout)?.trim().to_string()
    };

    // Require parent authentication
    auth::require_auth(|_token| {
        Box::pin(async move {
            let conn = Connection::system().await?;
            let proxy = FamilyDaemonProxy::new(&conn).await?;

            let response = proxy.get_weekly_report(&profile, &week_start).await?;

            // Parse response
            let result: serde_json::Value = serde_json::from_str(&response)?;

            if let Some(error) = result.get("error").and_then(|e| e.as_str()) {
                println!("Failed to get weekly report: {}", error);
                return Ok(());
            }

            // Display the report
            println!("\n📊 Weekly Activity Report (Week of {})", week_start);
            println!("═══════════════════════════════════════════════\n");

            if let Some(total_time) =
                result.get("total_screen_time_minutes").and_then(|s| s.as_u64())
            {
                let hours = total_time / 60;
                let minutes = total_time % 60;
                println!("⏱️  Total Screen Time: {}h {}m", hours, minutes);
            }

            if let Some(avg_time) = result.get("average_daily_minutes").and_then(|s| s.as_u64()) {
                let hours = avg_time / 60;
                let minutes = avg_time % 60;
                println!("📊 Daily Average: {}h {}m", hours, minutes);
            }

            if let Some(most_active) = result.get("most_active_day").and_then(|m| m.as_str()) {
                println!("🗓️  Most Active Day: {}", most_active);
            }

            if let Some(violations) = result.get("policy_violations").and_then(|v| v.as_u64()) {
                println!("⚠️  Policy Violations: {}", violations);
            }

            if let Some(edu_pct) = result.get("educational_percentage").and_then(|e| e.as_f64()) {
                println!("🎓 Educational Content: {:.1}%", edu_pct);
            }

            // Display category usage breakdown
            if let Some(categories) = result.get("top_categories").and_then(|c| c.as_array()) {
                if !categories.is_empty() {
                    println!("\n📁 Category Breakdown:");
                    println!("─────────────────────────────────────────────");
                    for (i, cat) in categories.iter().enumerate().take(10) {
                        let category =
                            cat.get("category").and_then(|c| c.as_str()).unwrap_or("Unknown");
                        let duration =
                            cat.get("duration_minutes").and_then(|d| d.as_u64()).unwrap_or(0);
                        let percentage =
                            cat.get("percentage").and_then(|p| p.as_f64()).unwrap_or(0.0);

                        let hours = duration / 60;
                        let minutes = duration % 60;

                        println!("  {}. {}", i + 1, category);
                        println!("     {}h {}m ({:.1}%)", hours, minutes, percentage);
                    }
                }
            }

            println!();
            Ok(())
        })
    })
    .await
}

/// Export reports to a file
pub async fn export(
    profile: &str,
    format: &str,
    start_date: &str,
    end_date: &str,
    output: Option<&str>,
) -> Result<()> {
    let profile = profile.to_string();
    let format = format.to_string();
    let start_date = start_date.to_string();
    let end_date = end_date.to_string();
    let output = output.map(|s| s.to_string());

    // Require parent authentication
    auth::require_auth(|_token| {
        Box::pin(async move {
            let conn = Connection::system().await?;
            let proxy = FamilyDaemonProxy::new(&conn).await?;

            let response = proxy.export_reports(&profile, &format, &start_date, &end_date).await?;

            // Parse response
            let result: serde_json::Value = serde_json::from_str(&response)?;

            if let Some(error) = result.get("error").and_then(|e| e.as_str()) {
                println!("Failed to export reports: {}", error);
                return Ok(());
            }

            // Write to file or stdout
            if let Some(output_path) = output {
                std::fs::write(&output_path, response)?;
                println!("✅ Report exported to: {}", output_path);
            } else {
                println!("{}", response);
            }

            Ok(())
        })
    })
    .await
}
