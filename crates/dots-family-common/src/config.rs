use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub data_dir: Option<String>,
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: None,
            log_level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbusConfig {
    pub service_name: String,
    pub object_path: String,
}

impl Default for DbusConfig {
    fn default() -> Self {
        Self {
            service_name: "org.dots.FamilyDaemon".to_string(),
            object_path: "/org/dots/FamilyDaemon".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_config_default() {
        let config = GeneralConfig::default();
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_dbus_config_default() {
        let config = DbusConfig::default();
        assert_eq!(config.service_name, "org.dots.FamilyDaemon");
    }
}
