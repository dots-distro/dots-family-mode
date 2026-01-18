use dots_family_daemon::profile_manager::ProfileManager;
use dots_family_db::Database;
use tempfile::tempdir;

async fn setup_test_db() -> (Database, tempfile::TempDir, dots_family_daemon::config::DaemonConfig)
{
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let daemon_config = dots_family_daemon::config::DaemonConfig {
        database: dots_family_daemon::config::DatabaseConfig {
            path: db_path.to_str().unwrap().to_string(),
            encryption_key: None,
        },
        auth: dots_family_daemon::config::AuthConfig { parent_password_hash: None },
    };

    let db_config = dots_family_db::DatabaseConfig {
        path: db_path.to_str().unwrap().to_string(),
        encryption_key: None,
    };

    let db = Database::new(db_config).await.unwrap();
    db.run_migrations().await.unwrap();
    (db, dir, daemon_config)
}

#[tokio::test]
async fn test_daily_report_returns_empty_for_nonexistent_profile() {
    let (db, _dir, config) = setup_test_db().await;
    let manager = ProfileManager::new(&config, db).await.unwrap();

    let result = manager.get_daily_report("nonexistent_profile", "2026-01-18").await;

    assert!(result.is_ok(), "Should return empty report for non-existent profile");

    let report = result.unwrap();
    assert_eq!(report.screen_time_minutes, 0);
    assert_eq!(report.top_activity, "No Activity");
    assert_eq!(report.top_category, "None");
    assert_eq!(report.violations, 0);
    assert_eq!(report.blocked_attempts, 0);
    assert!(report.apps_used.is_empty());
}

#[tokio::test]
async fn test_daily_report_with_invalid_date() {
    let (db, _dir, config) = setup_test_db().await;
    let manager = ProfileManager::new(&config, db).await.unwrap();

    let result = manager.get_daily_report("test_profile", "invalid-date").await;

    assert!(result.is_err(), "Should return error for invalid date format");
}

#[tokio::test]
async fn test_weekly_report_returns_empty_for_nonexistent_profile() {
    let (db, _dir, config) = setup_test_db().await;
    let manager = ProfileManager::new(&config, db).await.unwrap();

    let result = manager.get_weekly_report("nonexistent_profile", "2026-01-13").await;

    assert!(result.is_ok(), "Should return empty report for non-existent profile");

    let report = result.unwrap();
    assert_eq!(report.total_screen_time_minutes, 0);
    assert_eq!(report.average_daily_minutes, 0);
    assert_eq!(report.most_active_day, "No Activity");
    assert_eq!(report.policy_violations, 0);
    assert_eq!(report.educational_percentage, 0.0);
    assert!(report.top_categories.is_empty());
}

#[tokio::test]
async fn test_weekly_report_with_invalid_date() {
    let (db, _dir, config) = setup_test_db().await;
    let manager = ProfileManager::new(&config, db).await.unwrap();

    let result = manager.get_weekly_report("test_profile", "invalid-date").await;

    assert!(result.is_err(), "Should return error for invalid date format");
}

#[tokio::test]
async fn test_export_reports_json_format_success() {
    let (db, _dir, config) = setup_test_db().await;
    let manager = ProfileManager::new(&config, db).await.unwrap();

    let result = manager.export_reports("test_profile", "json", "2026-01-17", "2026-01-18").await;

    assert!(result.is_ok(), "Should succeed with JSON format");

    let export_json = result.unwrap();
    assert!(!export_json.is_empty(), "Export should not be empty");

    let parsed: serde_json::Value =
        serde_json::from_str(&export_json).expect("Export should be valid JSON");

    assert!(parsed.is_array(), "Export should be an array of daily reports");
    let reports = parsed.as_array().unwrap();
    assert_eq!(reports.len(), 2, "Should have reports for 2 days");
}

#[tokio::test]
async fn test_export_reports_csv_format_success() {
    let (db, _dir, config) = setup_test_db().await;
    let manager = ProfileManager::new(&config, db).await.unwrap();

    let result = manager.export_reports("test_profile", "csv", "2026-01-17", "2026-01-18").await;

    assert!(result.is_ok(), "Should succeed with CSV format");

    let export_csv = result.unwrap();
    assert!(!export_csv.is_empty(), "Export should not be empty");

    let lines: Vec<&str> = export_csv.lines().collect();
    assert!(lines.len() >= 3, "Should have header + 2 data lines");
    assert!(
        lines[0].contains("Date") && lines[0].contains("Screen Time"),
        "CSV should have proper header"
    );
    assert!(lines[1].contains("2026-01-17"), "Should contain first day data");
    assert!(lines[2].contains("2026-01-18"), "Should contain second day data");
}

#[tokio::test]
async fn test_export_reports_with_invalid_dates() {
    let (db, _dir, config) = setup_test_db().await;
    let manager = ProfileManager::new(&config, db).await.unwrap();

    let result = manager.export_reports("test_profile", "json", "invalid-date", "2026-01-18").await;

    assert!(result.is_err(), "Should return error for invalid start date");

    let result = manager.export_reports("test_profile", "json", "2026-01-17", "invalid-date").await;

    assert!(result.is_err(), "Should return error for invalid end date");
}

#[tokio::test]
async fn test_export_reports_with_unsupported_format() {
    let (db, _dir, config) = setup_test_db().await;
    let manager = ProfileManager::new(&config, db).await.unwrap();

    let result = manager.export_reports("test_profile", "xml", "2026-01-17", "2026-01-18").await;

    assert!(result.is_err(), "Should return error for unsupported export format");
}
