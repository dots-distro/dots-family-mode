use anyhow::Result;
use dots_family_proto::daemon::FamilyDaemonProxy;
use zbus::Connection;

pub async fn view() -> Result<()> {
    let conn = Connection::session().await?;
    let proxy = FamilyDaemonProxy::new(&conn).await?;

    let profile_json = proxy.get_active_profile().await?;
    let profile: serde_json::Value = serde_json::from_str(&profile_json)?;

    if profile.get("error").is_some() {
        println!("No active session");
        return Ok(());
    }

    let name = profile["name"].as_str().unwrap_or("unknown");
    let remaining_minutes = proxy.get_remaining_time().await?;

    println!("Active Session:");
    println!("  Profile: {}", name);
    println!("  Remaining time: {} minutes", remaining_minutes);

    Ok(())
}

pub async fn history(_profile_id: Option<&str>) -> Result<()> {
    println!("Session history:");
    println!("  (Requires database queries - to be implemented)");
    Ok(())
}
