use zbus::proxy;

#[proxy(
    interface = "org.dots.FamilyDaemon",
    default_service = "org.dots.FamilyDaemon",
    default_path = "/org/dots/FamilyDaemon"
)]
pub trait FamilyDaemon {
    async fn get_active_profile(&self) -> zbus::Result<String>;

    async fn check_application_allowed(&self, app_id: &str) -> zbus::Result<bool>;

    async fn get_remaining_time(&self) -> zbus::Result<u32>;

    async fn report_activity(&self, activity_json: &str) -> zbus::Result<String>;

    async fn send_heartbeat(&self, monitor_id: &str) -> zbus::Result<String>;

    async fn ping(&self) -> zbus::Result<String>;

    async fn authenticate_parent(&self, password: &str) -> zbus::Result<String>;

    async fn validate_session(&self, token: &str) -> zbus::Result<bool>;

    async fn revoke_session(&self, token: &str) -> zbus::Result<bool>;

    async fn list_profiles(&self) -> zbus::Result<String>;

    async fn create_profile(
        &self,
        name: &str,
        age_group: &str,
        username: &str,
    ) -> zbus::Result<String>;

    async fn set_active_profile(&self, profile_id: &str) -> zbus::Result<()>;

    async fn request_parent_permission(
        &self,
        request_type: &str,
        details: &str,
        token: &str,
    ) -> zbus::Result<String>;

    async fn request_command_approval(
        &self,
        command: &str,
        risk_level: &str,
        reasons: &str,
    ) -> zbus::Result<String>;

    async fn check_app_policy(&self, app_id: &str) -> zbus::Result<String>;

    async fn process_activity_for_policy(&self, activity_json: &str) -> zbus::Result<String>;

    async fn sync_profile_to_policy(&self, profile_id: &str) -> zbus::Result<String>;

    // Report generation methods
    async fn get_daily_report(&self, profile_id: &str, date: &str) -> zbus::Result<String>;

    async fn get_weekly_report(&self, profile_id: &str, week_start: &str) -> zbus::Result<String>;

    async fn export_reports(
        &self,
        profile_id: &str,
        format: &str,
        start_date: &str,
        end_date: &str,
    ) -> zbus::Result<String>;

    #[zbus(signal)]
    async fn policy_updated(&self, profile_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn time_limit_warning(&self, minutes_remaining: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn tamper_detected(&self, reason: &str) -> zbus::Result<()>;
}
