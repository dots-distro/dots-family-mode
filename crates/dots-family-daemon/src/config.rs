use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DaemonConfig {
    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub auth: AuthConfig,

    #[serde(default)]
    pub dbus: DbusConfig,

    #[serde(default)]
    pub dry_run: Option<bool>,

    /// Users allowed to call mutating D-Bus methods.
    ///
    /// This mirrors the NixOS `services.dots-family.parentUsers` option. The
    /// daemon uses it as a second gate behind the D-Bus policy: root and these
    /// users pass, every other caller is refused. Names are resolved to UIDs
    /// at start; a name that does not resolve grants nothing.
    ///
    /// Read from `DOTS_FAMILY_PARENT_USERS` (comma separated) when the key is
    /// absent from the configuration file.
    ///
    /// SECURITY: an empty list means only root may call mutating methods. It
    /// fails closed on purpose. A deployment that manages profiles must
    /// configure its parents.
    #[serde(default = "parent_users_from_env", alias = "parentUsers")]
    pub parent_users: Vec<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            auth: AuthConfig::default(),
            dbus: DbusConfig::default(),
            dry_run: None,
            parent_users: parent_users_from_env(),
        }
    }
}

/// Read the parent user names from `DOTS_FAMILY_PARENT_USERS`.
///
/// The value is a comma-separated list. Empty entries are ignored.
fn parent_users_from_env() -> Vec<String> {
    std::env::var("DOTS_FAMILY_PARENT_USERS")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub path: String,
    /// SQLCipher key material. This is never serialised; the key lives in a
    /// separate root-only file instead (see [`DatabaseConfig::encryption_key_path`]).
    #[serde(skip)]
    pub encryption_key: Option<String>,
}

impl DatabaseConfig {
    /// Path of the root-only file that stores the SQLCipher key.
    ///
    /// Overridable with `DOTS_FAMILY_DB_ENCRYPTION_KEY_PATH`. The default is
    /// `db.key` next to the database, which places it in
    /// `/var/lib/dots-family/db.key` for the packaged daemon.
    pub fn encryption_key_path(&self) -> PathBuf {
        if let Some(path) = std::env::var_os("DOTS_FAMILY_DB_ENCRYPTION_KEY_PATH") {
            return PathBuf::from(path);
        }

        Path::new(&self.path)
            .parent()
            .map(|dir| dir.join("db.key"))
            .unwrap_or_else(|| PathBuf::from("/var/lib/dots-family/db.key"))
    }

    /// Persist the key to [`DatabaseConfig::encryption_key_path`].
    ///
    /// The file is created with mode `0600` and its parent with mode `0700`.
    /// The key is never logged, returned, or written to the TOML config.
    pub fn write_encryption_key(&self, key: &str) -> Result<()> {
        let path = self.encryption_key_path();

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create key directory: {:?}", parent))?;
                set_mode(parent, 0o700)?;
            }
        }

        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        options.mode(0o600);

        let mut file =
            options.open(&path).with_context(|| format!("Failed to open key file: {:?}", path))?;
        set_mode(&path, 0o600)?;
        file.write_all(key.as_bytes())
            .with_context(|| format!("Failed to write key file: {:?}", path))?;
        file.write_all(b"\n").with_context(|| format!("Failed to write key file: {:?}", path))?;
        file.sync_all().with_context(|| format!("Failed to flush key file: {:?}", path))?;

        Ok(())
    }

    /// Load the key from the key file, if one is present.
    pub fn read_encryption_key(&self) -> Option<String> {
        let contents = fs::read_to_string(self.encryption_key_path()).ok()?;
        let key = contents.trim();
        if key.is_empty() {
            None
        } else {
            Some(key.to_string())
        }
    }
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("Failed to set permissions on {:?}", path))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let path = std::env::var("DOTS_FAMILY_DB_PATH").unwrap_or_else(|_| {
            let config_dir =
                dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("dots-family");
            config_dir.join("family.db").to_string_lossy().to_string()
        });

        Self { path, encryption_key: None }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AuthConfig {
    pub parent_password_hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DbusConfig {
    pub service_name: String,
    pub use_session_bus: bool,
}

impl Default for DbusConfig {
    fn default() -> Self {
        // SECURITY: Daemon must always run on system bus for security
        // User services connect to system bus daemon for privileged operations
        Self {
            service_name: "org.dots.FamilyDaemon".to_string(),
            use_session_bus: false, // Always use system bus for security
        }
    }
}

impl DaemonConfig {
    /// Default configuration file path
    pub fn default_config_path() -> PathBuf {
        // Check for service environment variable first
        if let Ok(config_dir) = std::env::var("DOTS_FAMILY_CONFIG_DIR") {
            return PathBuf::from(config_dir).join("daemon.toml");
        }

        // Fall back to user config directory for development
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

        let mut config: DaemonConfig = toml::from_str(&config_content)
            .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

        // The key is deliberately not stored in the TOML file; load it from the
        // separate root-only file instead.
        config.database.encryption_key = config.database.read_encryption_key();

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
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        options.mode(0o600);

        let mut file = options
            .open(config_path)
            .with_context(|| format!("Failed to write config file: {:?}", config_path))?;
        file.write_all(config_content.as_bytes())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_key_is_never_serialised() {
        let config = DatabaseConfig {
            path: "/var/lib/dots-family/family.db".to_string(),
            encryption_key: Some("x'00112233445566778899aabbccddeeff'".to_string()),
        };

        let serialised = toml::to_string(&config).unwrap();

        assert!(!serialised.contains("encryption_key"), "key field leaked: {serialised}");
        assert!(!serialised.contains("0011223344556677"), "key material leaked: {serialised}");
        assert!(serialised.contains("family.db"));
    }

    #[test]
    fn key_path_defaults_next_to_the_database() {
        let config = DatabaseConfig {
            path: "/var/lib/dots-family/family.db".to_string(),
            encryption_key: None,
        };

        assert_eq!(config.encryption_key_path(), PathBuf::from("/var/lib/dots-family/db.key"));
    }


    #[cfg(unix)]
    #[test]
    fn the_config_file_is_written_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let unique = format!(
            "dots-family-daemon-config-test-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);

        DaemonConfig::default().save_to_path(&path).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;

        fs::remove_file(&path).unwrap();

        assert_eq!(mode, 0o600, "config file mode was {:o}", mode);
    }
}
