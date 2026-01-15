use anyhow::Result;
use dots_family_proto::daemon::FamilyDaemonProxy;
use zbus::Connection;

pub async fn application(app_id: &str) -> Result<()> {
    let conn = Connection::system().await?;
    let proxy = FamilyDaemonProxy::new(&conn).await?;

    let allowed = proxy.check_application_allowed(app_id).await?;

    if allowed {
        println!("✓ Application '{}' is ALLOWED", app_id);
    } else {
        println!("✗ Application '{}' is BLOCKED", app_id);
    }

    Ok(())
}
