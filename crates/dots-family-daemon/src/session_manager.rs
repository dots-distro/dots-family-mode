use anyhow::Result;
use dots_family_db::queries::SessionQueries;
use dots_family_db::{Database, NewSession};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct _SessionManager {
    db: Database,
    active_session: Arc<RwLock<Option<String>>>,
}

impl _SessionManager {
    pub fn _new(db: Database) -> Self {
        Self { db, active_session: Arc::new(RwLock::new(None)) }
    }

    pub async fn _start_session(&self, profile_id: &str) -> Result<String> {
        let new_session = NewSession::new(profile_id.to_string());
        let session_id = new_session.id.clone();

        SessionQueries::create(&self.db, new_session).await?;

        let mut active = self.active_session.write().await;
        *active = Some(session_id.clone());

        info!("Session started: {} for profile {}", session_id, profile_id);
        Ok(session_id)
    }

    pub async fn _end_session(&self, reason: &str) -> Result<()> {
        let session_id = {
            let active = self.active_session.read().await;
            active.clone()
        };

        let Some(session_id) = session_id else {
            return Ok(());
        };

        SessionQueries::end_session(&self.db, &session_id, reason, 0, 0, 0, 0).await?;

        let mut active = self.active_session.write().await;
        *active = None;

        info!("Session ended: {}", session_id);
        Ok(())
    }

    pub async fn _get_active_session(&self) -> Result<Option<String>> {
        let session = self.active_session.read().await;
        Ok(session.clone())
    }
}
