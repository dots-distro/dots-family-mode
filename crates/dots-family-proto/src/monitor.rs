use zbus::proxy;

#[proxy(
    interface = "org.dots.FamilyMonitor",
    default_service = "org.dots.FamilyMonitor",
    default_path = "/org/dots/FamilyMonitor"
)]
pub trait FamilyMonitor {
    async fn get_current_activity(&self) -> zbus::Result<String>;

    async fn get_active_window(&self) -> zbus::Result<String>;

    #[zbus(signal)]
    async fn activity_changed(&self, activity_json: &str) -> zbus::Result<()>;
}
