use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilterConfig {
    pub proxy: ProxyConfig,
    pub filtering: FilteringConfig,
    pub daemon: DaemonConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyConfig {
    pub bind_address: String,
    pub port: u16,
    pub upstream_timeout_seconds: u64,
    pub max_body_size_mb: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilteringConfig {
    pub enabled: bool,
    pub safe_search_enforcement: bool,
    pub block_categories: Vec<String>,
    pub allow_categories: Vec<String>,
    pub blocked_domains: Vec<String>,
    pub allowed_domains: Vec<String>,
    pub custom_rules_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DaemonConfig {
    pub dbus_interface: String,
    pub check_permissions: bool,
    pub log_activity: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig {
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
                upstream_timeout_seconds: 30,
                max_body_size_mb: 10,
            },
            filtering: FilteringConfig {
                enabled: true,
                safe_search_enforcement: true,
                block_categories: vec![
                    "adult".to_string(),
                    "violence".to_string(),
                    "gambling".to_string(),
                    "drugs".to_string(),
                ],
                allow_categories: vec![
                    "educational".to_string(),
                    "children".to_string(),
                    "reference".to_string(),
                ],
                blocked_domains: vec![],
                allowed_domains: vec![],
                custom_rules_enabled: true,
            },
            daemon: DaemonConfig {
                dbus_interface: "org.dots.FamilyDaemon".to_string(),
                check_permissions: true,
                log_activity: true,
            },
        }
    }
}

impl FilterConfig {
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("dots-family")
            .join("filter.toml")
    }

    pub fn load(config_path: Option<String>) -> Result<Self> {
        let config_path = config_path.map(PathBuf::from).unwrap_or_else(Self::default_config_path);

        debug!("Loading filter configuration from {:?}", config_path);

        if !config_path.exists() {
            info!(
                "Configuration file not found at {:?}, creating default configuration",
                config_path
            );
            let default_config = Self::default();
            default_config.save_to_path(&config_path)?;
            return Ok(default_config);
        }

        let config_content = fs::read_to_string(&config_path)?;
        let config: FilterConfig = toml::from_str(&config_content)?;

        info!("Loaded filter configuration from {:?}", config_path);
        Ok(config)
    }

    pub fn save_to_path(&self, config_path: &Path) -> Result<()> {
        debug!("Saving filter configuration to {:?}", config_path);

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let config_content = toml::to_string_pretty(self)?;
        fs::write(config_path, config_content)?;

        info!("Saved filter configuration to {:?}", config_path);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        if self.proxy.port == 0 {
            return Err(anyhow::anyhow!("Proxy port cannot be 0"));
        }

        if self.proxy.bind_address.is_empty() {
            return Err(anyhow::anyhow!("Bind address cannot be empty"));
        }

        if !self.filtering.enabled {
            warn!("Content filtering is disabled - all web traffic will be allowed");
        }

        debug!("Filter configuration validation passed");
        Ok(())
    }
}
