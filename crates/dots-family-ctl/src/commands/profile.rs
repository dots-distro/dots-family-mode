use crate::auth;
use anyhow::Result;
use dots_family_proto::daemon::FamilyDaemonProxy;
use zbus::Connection;

pub async fn list() -> Result<()> {
    let conn = Connection::session().await?;
    let proxy = FamilyDaemonProxy::new(&conn).await?;

    let profiles_json = proxy.list_profiles().await?;
    let profiles: Vec<serde_json::Value> = serde_json::from_str(&profiles_json)?;

    println!("Available profiles:");
    for profile in profiles {
        let name = profile["name"].as_str().unwrap_or("unknown");
        let age_group = profile["age_group"].as_str().unwrap_or("unknown");
        let active = profile["active"].as_bool().unwrap_or(false);
        let active_marker = if active { " (active)" } else { "" };
        println!("  - {} ({}){}", name, age_group, active_marker);
    }

    Ok(())
}

pub async fn show(_name: &str) -> Result<()> {
    let conn = Connection::session().await?;
    let proxy = FamilyDaemonProxy::new(&conn).await?;

    let profile_json = proxy.get_active_profile().await?;
    println!("Profile data: {}", profile_json);

    Ok(())
}

pub async fn create(name: &str, age_group: &str) -> Result<()> {
    let conn = Connection::session().await?;
    let proxy = FamilyDaemonProxy::new(&conn).await?;

    // Try to create profile directly first (for initial setup when no parent password exists)
    let profile_id = proxy.create_profile(name, age_group).await?;

    if let Some(error_msg) = profile_id.strip_prefix("error:") {
        // If direct creation failed, it might be due to authentication requirement
        if error_msg.contains("authentication not configured") {
            println!("This appears to be the first-time setup. Creating initial profile...");
        } else {
            // For other errors, try with authentication
            println!("Profile creation requires parent authentication.");
            let name = name.to_string();
            let age_group = age_group.to_string();

            auth::require_auth_simple(|| {
                Box::pin(async move {
                    let conn = Connection::session().await?;
                    let proxy = FamilyDaemonProxy::new(&conn).await?;

                    let profile_id = proxy.create_profile(&name, &age_group).await?;

                    if let Some(error_msg) = profile_id.strip_prefix("error:") {
                        println!("Failed to create profile: {}", error_msg);
                    } else {
                        println!("Created profile '{}' with ID: {}", name, profile_id);
                    }

                    Ok(())
                })
            })
            .await?;
            return Ok(());
        }
    }

    if !profile_id.starts_with("error:") {
        println!("Created profile '{}' with ID: {}", name, profile_id);
    } else {
        println!("Failed to create profile: {}", profile_id);
    }

    Ok(())
}

pub async fn set_active(profile_id: &str) -> Result<()> {
    let profile_id = profile_id.to_string();

    auth::require_auth_simple(|| {
        Box::pin(async move {
            let conn = Connection::session().await?;
            let proxy = FamilyDaemonProxy::new(&conn).await?;

            proxy.set_active_profile(&profile_id).await?;
            println!("Set active profile to: {}", profile_id);

            Ok(())
        })
    })
    .await
}
