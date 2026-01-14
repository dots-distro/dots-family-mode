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
    pub fn load() -> Result<Self> {
        Ok(Self::default())
    }
}
