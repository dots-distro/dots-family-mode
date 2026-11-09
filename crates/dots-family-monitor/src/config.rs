use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitorConfig {
    #[serde(default = "default_polling_interval")]
    pub polling_interval_ms: u64,

    #[serde(default = "default_idle_threshold")]
    pub report_idle_threshold_seconds: u64,
}

fn default_polling_interval() -> u64 {
    1000
}

fn default_idle_threshold() -> u64 {
    60
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self { polling_interval_ms: 1000, report_idle_threshold_seconds: 60 }
    }
}

impl MonitorConfig {
    /// Load the monitor configuration from the environment.
    ///
    /// `ACTIVITY_REPORTING_INTERVAL` is the polling interval in milliseconds.
    /// A missing value or one that is not a valid `u64` keeps the default.
    pub fn load() -> Result<Self> {
        let mut config = Self::default();
        if let Ok(raw) = std::env::var("ACTIVITY_REPORTING_INTERVAL") {
            if let Ok(interval_ms) = raw.trim().parse::<u64>() {
                config.polling_interval_ms = interval_ms;
            }
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_reporting_interval_is_read_from_environment() {
        std::env::set_var("ACTIVITY_REPORTING_INTERVAL", "250");
        let config = MonitorConfig::load().expect("config should load");
        assert_eq!(config.polling_interval_ms, 250);

        std::env::remove_var("ACTIVITY_REPORTING_INTERVAL");
        let config = MonitorConfig::load().expect("config should load");
        assert_eq!(config.polling_interval_ms, 1000);
    }
}
