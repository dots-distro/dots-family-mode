use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DaemonConfig {
    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub encryption_key: Option<String>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let config_dir =
            dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("dots-family");

        Self {
            path: config_dir.join("family.db").to_string_lossy().to_string(),
            encryption_key: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AuthConfig {
    pub parent_password_hash: Option<String>,
}

impl DaemonConfig {
    /// Default configuration file path
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("dots-family")
            .join("daemon.toml")
    }

    /// Load configuration from file, creating default if it doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::default_config_path();
        Self::load_from_path(&config_path)
    }

    /// Load configuration from a specific path
    pub fn load_from_path(config_path: &Path) -> Result<Self> {
        debug!("Loading daemon configuration from {:?}", config_path);

        if !config_path.exists() {
            info!(
                "Configuration file not found at {:?}, creating default configuration",
                config_path
            );
            let default_config = Self::default();
            default_config.save_to_path(config_path)?;
            return Ok(default_config);
        }

        let config_content = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        let config: DaemonConfig = toml::from_str(&config_content)
            .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

        info!("Loaded daemon configuration from {:?}", config_path);
        Ok(config)
    }

    /// Save configuration to the default file path
    #[allow(dead_code)] // Will be used by CLI/GUI applications
    pub fn save(&self) -> Result<()> {
        let config_path = Self::default_config_path();
        self.save_to_path(&config_path)
    }

    /// Save configuration to a specific path
    pub fn save_to_path(&self, config_path: &Path) -> Result<()> {
        debug!("Saving daemon configuration to {:?}", config_path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let config_content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration to TOML")?;

        fs::write(config_path, config_content)
            .with_context(|| format!("Failed to write config file: {:?}", config_path))?;

        info!("Saved daemon configuration to {:?}", config_path);
        Ok(())
    }

    /// Update the configuration in memory and save to disk
    #[allow(dead_code)] // Will be used by CLI/GUI applications
    pub fn update_and_save<F>(&mut self, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut Self),
    {
        update_fn(self);
        self.save()
    }

    /// Validate the configuration settings
    #[allow(dead_code)] // Will be used by CLI/GUI applications
    pub fn validate(&self) -> Result<()> {
        // Validate database path can be created
        if let Some(parent) = Path::new(&self.database.path).parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create database directory: {:?}", parent))?;
        }

        // Warn about security issues
        if self.auth.parent_password_hash.is_none() {
            warn!("No parent password hash configured - authentication will fail");
        }

        if self.database.encryption_key.is_none() {
            warn!("Database encryption is disabled - family data will be stored in plaintext");
        }

        debug!("Configuration validation passed");
        Ok(())
    }
}
