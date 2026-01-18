use chrono::Utc;
use dots_family_common::types::{
    AgeGroup, ApplicationConfig, ApplicationMode, Profile, ProfileConfig, ScreenTimeConfig,
    TerminalFilteringConfig, TimeWindows, WebFilteringConfig,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ProfileStore {
    pub profiles: Vec<Profile>,
    pub selected_profile_id: Option<Uuid>,
}

impl ProfileStore {
    pub fn new() -> Self {
        // Create some dummy data for now (necessary for testing/mocking)
        let mut profiles = Vec::new();

        let p1 = Profile {
            id: Uuid::new_v4(),
            name: "Alice".to_string(),
            age_group: AgeGroup::LateElementary,
            birthday: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            config: ProfileConfig {
                screen_time: ScreenTimeConfig {
                    daily_limit_minutes: 120,
                    weekend_bonus_minutes: 60,
                    exempt_categories: vec![],
                    windows: TimeWindows { weekday: vec![], weekend: vec![] },
                },
                applications: ApplicationConfig {
                    mode: ApplicationMode::Blocklist,
                    allowed: vec![],
                    blocked: vec![],
                    blocked_categories: vec![],
                },
                web_filtering: WebFilteringConfig {
                    enabled: true,
                    safe_search: true,
                    blocked_categories: vec![],
                    allowed_domains: vec![],
                    blocked_domains: vec![],
                },
                terminal_filtering: TerminalFilteringConfig::default(),
            },
            active: true,
        };

        profiles.push(p1);

        Self { profiles, selected_profile_id: None }
    }

    pub fn get_selected_profile(&self) -> Option<&Profile> {
        self.selected_profile_id.and_then(|id| self.profiles.iter().find(|p| p.id == id))
    }

    pub fn update_profile(&mut self, profile: Profile) {
        if let Some(index) = self.profiles.iter().position(|p| p.id == profile.id) {
            self.profiles[index] = profile;
        } else {
            self.profiles.push(profile);
        }
    }
}
