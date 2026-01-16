use dots_family_db::{Database, DatabaseConfig};
use dots_family_proto::daemon::FamilyDaemonProxy;
use std::sync::Arc;
use tokio::sync::RwLock;
use zbus::{interface, ConnectionBuilder};

#[derive(Debug, Clone)]
struct MockDaemonState {
    active_profile_json: String,
    remaining_time: u32,
}

struct MockDaemon {
    state: Arc<RwLock<MockDaemonState>>,
    _db: Arc<Database>,
}

#[interface(name = "org.dots.FamilyDaemon")]
impl MockDaemon {
    async fn get_active_profile(&self) -> String {
        let state = self.state.read().await;
        state.active_profile_json.clone()
    }

    async fn check_application_allowed(&self, app_id: &str) -> bool {
        !app_id.is_empty()
    }

    async fn get_remaining_time(&self) -> u32 {
        let state = self.state.read().await;
        state.remaining_time
    }

    async fn report_activity(&self, activity_json: &str) -> String {
        let _activity = activity_json;
        "ok".to_string()
    }

    async fn authenticate_parent(&self, _password: &str) -> String {
        "mock_token".to_string()
    }

    async fn validate_session(&self, _token: &str) -> bool {
        true
    }

    async fn revoke_session(&self, _token: &str) -> bool {
        true
    }

    async fn send_heartbeat(&self, _monitor_id: &str) -> String {
        "ok".to_string()
    }

    async fn list_profiles(&self) -> String {
        "[]".to_string()
    }

    async fn create_profile(&self, _name: &str, _age_group: &str) -> String {
        "mock_profile_id".to_string()
    }

    async fn set_active_profile(&self, _profile_id: &str) {}
}

impl MockDaemon {
    fn new(db: Arc<Database>) -> Self {
        Self {
            state: Arc::new(RwLock::new(MockDaemonState {
                active_profile_json: "null".to_string(),
                remaining_time: 7200,
            })),
            _db: db,
        }
    }
}

#[tokio::test]
async fn test_mock_daemon_basic() {
    let config = DatabaseConfig { path: ":memory:".to_string(), encryption_key: None };

    let db = Database::new(config).await.unwrap();
    db.run_migrations().await.unwrap();

    let db = Arc::new(db);
    let daemon = MockDaemon::new(db.clone());

    let conn = ConnectionBuilder::session()
        .unwrap()
        .name("org.dots.FamilyDaemon.Test")
        .unwrap()
        .serve_at("/org/dots/FamilyDaemon", daemon)
        .unwrap()
        .build()
        .await
        .unwrap();

    let proxy = FamilyDaemonProxy::builder(&conn)
        .destination("org.dots.FamilyDaemon.Test")
        .unwrap()
        .path("/org/dots/FamilyDaemon")
        .unwrap()
        .build()
        .await
        .unwrap();

    let profile = proxy.get_active_profile().await.unwrap();
    assert_eq!(profile, "null");

    let allowed = proxy.check_application_allowed("firefox").await.unwrap();
    assert!(allowed);

    let remaining = proxy.get_remaining_time().await.unwrap();
    assert_eq!(remaining, 7200);

    let token = proxy.authenticate_parent("password").await.unwrap();
    assert_eq!(token, "mock_token");

    proxy.report_activity(r#"{"app":"firefox","duration":60}"#).await.unwrap();
}

#[tokio::test]
async fn test_mock_daemon_with_database() {
    use dots_family_db::models::NewProfile;
    use dots_family_db::queries::ProfileQueries;

    let config = DatabaseConfig { path: ":memory:".to_string(), encryption_key: None };

    let db = Database::new(config).await.unwrap();
    db.run_migrations().await.unwrap();

    let profile = NewProfile::new(
        "TestChild".to_string(),
        "8-12".to_string(),
        r#"{"screen_time": {"daily_limit_minutes": 120}}"#.to_string(),
    );

    let created = ProfileQueries::create(&db, profile).await.unwrap();
    assert_eq!(created.name, "TestChild");

    let fetched = ProfileQueries::get_by_id(&db, &created.id).await.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, "TestChild");

    let all_profiles = ProfileQueries::list_all(&db).await.unwrap();
    assert_eq!(all_profiles.len(), 1);
}
