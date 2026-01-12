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

    async fn report_activity(&self, activity_json: &str) -> zbus::Result<()>;

    async fn authenticate_parent(&self, password: &str) -> zbus::Result<String>;

    #[zbus(signal)]
    async fn policy_updated(&self, profile_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn time_limit_warning(&self, minutes_remaining: u32) -> zbus::Result<()>;
}
