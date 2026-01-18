use anyhow::Result;
use dots_family_proto::daemon::FamilyDaemonProxy;
use zbus::Connection;

pub async fn show() -> Result<()> {
    let conn = Connection::system().await?;
    let proxy = FamilyDaemonProxy::new(&conn).await?;

    let profile_json = proxy.get_active_profile().await?;
    let remaining = proxy.get_remaining_time().await?;

    println!("DOTS Family Mode Status");
    println!("=======================");
    println!();
    println!("Active Profile:");
    println!("{}", profile_json);
    println!();
    println!("Remaining Time: {} minutes", remaining);

    Ok(())
}
