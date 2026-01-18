use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Enable terminal command filtering
    pub enabled: bool,

    /// Commands to always block
    pub blocked_commands: HashSet<String>,

    /// Commands that require parent approval
    pub restricted_commands: HashSet<String>,

    /// Commands that are always allowed (whitelist)
    pub allowed_commands: HashSet<String>,

    /// File paths that are protected from modification
    pub protected_paths: HashSet<String>,

    /// Whether to log all commands to the daemon
    pub log_all_commands: bool,

    /// Whether to check with daemon for profile restrictions
    pub check_profile_restrictions: bool,

    /// DBus interface configuration
    pub daemon_interface: String,

    /// Risk analysis settings
    pub risk_analysis: RiskAnalysisConfig,

    /// Shell wrapper settings
    pub shell: ShellConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAnalysisConfig {
    /// Enable risk analysis
    pub enabled: bool,

    /// Block commands above this risk level
    pub block_threshold: String, // "low", "medium", "high", "critical"

    /// Require approval for commands above this risk level  
    pub approval_threshold: String,

    /// Maximum command length before flagging as suspicious
    pub max_command_length: usize,

    /// Patterns that indicate high risk
    pub high_risk_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Default shell to wrap (e.g., "/bin/bash")
    pub default_shell: String,

    /// Whether to show safety warnings
    pub show_warnings: bool,

    /// Timeout for parent approval requests (seconds)
    pub approval_timeout: u64,

    /// Whether to allow bypass with parent password
    pub allow_password_bypass: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        let mut blocked_commands = HashSet::new();
        blocked_commands.insert("rm -rf /".to_string());
        blocked_commands.insert("dd if=/dev/zero of=/dev/sda".to_string());
        blocked_commands.insert("mkfs".to_string());
        blocked_commands.insert("fdisk".to_string());
        blocked_commands.insert("cfdisk".to_string());
        blocked_commands.insert("parted".to_string());

        let mut restricted_commands = HashSet::new();
        restricted_commands.insert("sudo".to_string());
        restricted_commands.insert("su".to_string());
        restricted_commands.insert("passwd".to_string());
        restricted_commands.insert("userdel".to_string());
        restricted_commands.insert("usermod".to_string());
        restricted_commands.insert("chmod 777".to_string());
        restricted_commands.insert("chown".to_string());
        restricted_commands.insert("systemctl".to_string());
        restricted_commands.insert("service".to_string());
        restricted_commands.insert("mount".to_string());
        restricted_commands.insert("umount".to_string());

        let mut allowed_commands = HashSet::new();
        allowed_commands.insert("ls".to_string());
        allowed_commands.insert("cd".to_string());
        allowed_commands.insert("pwd".to_string());
        allowed_commands.insert("echo".to_string());
        allowed_commands.insert("cat".to_string());
        allowed_commands.insert("less".to_string());
        allowed_commands.insert("more".to_string());
        allowed_commands.insert("head".to_string());
        allowed_commands.insert("tail".to_string());
        allowed_commands.insert("grep".to_string());
        allowed_commands.insert("find".to_string());
        allowed_commands.insert("which".to_string());
        allowed_commands.insert("man".to_string());
        allowed_commands.insert("help".to_string());
        allowed_commands.insert("history".to_string());

        let mut protected_paths = HashSet::new();
        protected_paths.insert("/etc".to_string());
        protected_paths.insert("/boot".to_string());
        protected_paths.insert("/sys".to_string());
        protected_paths.insert("/proc".to_string());
        protected_paths.insert("/dev".to_string());
        protected_paths.insert("/root".to_string());
        protected_paths.insert("/var/log".to_string());

        let high_risk_patterns = vec![
            r"rm\s+-rf\s+/".to_string(),
            r"dd\s+if=/dev/".to_string(),
            r"mkfs\s+".to_string(),
            r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\};\s*:".to_string(), // Fork bomb
            r"curl.*\|\s*sh".to_string(),                         // Pipe to shell
            r"wget.*-O.*-".to_string(),                           // Wget pipe
            r"sudo\s+chmod\s+777".to_string(),
        ];

        Self {
            enabled: true,
            blocked_commands,
            restricted_commands,
            allowed_commands,
            protected_paths,
            log_all_commands: true,
            check_profile_restrictions: true,
            daemon_interface: "org.dots.FamilyDaemon".to_string(),
            risk_analysis: RiskAnalysisConfig {
                enabled: true,
                block_threshold: "critical".to_string(),
                approval_threshold: "high".to_string(),
                max_command_length: 1000,
                high_risk_patterns,
            },
            shell: ShellConfig {
                default_shell: "/bin/bash".to_string(),
                show_warnings: true,
                approval_timeout: 300, // 5 minutes
                allow_password_bypass: true,
            },
        }
    }
}

impl TerminalConfig {
    /// Load configuration from file or use defaults
    pub fn load() -> Result<Self> {
        // For now, use defaults. In the future, load from:
        // - /etc/dots/terminal-filter.toml
        // - ~/.config/dots/terminal-filter.toml
        Ok(Self::default())
    }

    /// Check if a command is explicitly blocked
    pub fn is_blocked_command(&self, command: &str) -> bool {
        self.blocked_commands
            .iter()
            .any(|blocked| command.starts_with(blocked) || command.contains(blocked))
    }

    /// Check if a command requires approval
    #[allow(dead_code)]
    pub fn is_restricted_command(&self, command: &str) -> bool {
        self.restricted_commands
            .iter()
            .any(|restricted| command.starts_with(restricted) || command.contains(restricted))
    }

    /// Check if a command is explicitly allowed
    pub fn is_allowed_command(&self, command: &str) -> bool {
        // Extract the first word (the actual command)
        let cmd = command.split_whitespace().next().unwrap_or("");
        self.allowed_commands.contains(cmd)
    }

    /// Check if a path is protected
    #[allow(dead_code)]
    pub fn is_protected_path(&self, path: &str) -> bool {
        self.protected_paths.iter().any(|protected| path.starts_with(protected))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_commands() {
        let config = TerminalConfig::default();

        assert!(config.is_blocked_command("rm -rf /"));
        assert!(config.is_blocked_command("sudo rm -rf /home"));
        assert!(!config.is_blocked_command("ls -la"));
    }

    #[test]
    fn test_restricted_commands() {
        let config = TerminalConfig::default();

        assert!(config.is_restricted_command("sudo systemctl restart"));
        assert!(config.is_restricted_command("passwd user"));
        assert!(!config.is_restricted_command("cat file.txt"));
    }

    #[test]
    fn test_allowed_commands() {
        let config = TerminalConfig::default();

        assert!(config.is_allowed_command("ls"));
        assert!(config.is_allowed_command("cd /home"));
        assert!(!config.is_allowed_command("sudo"));
    }

    #[test]
    fn test_protected_paths() {
        let config = TerminalConfig::default();

        assert!(config.is_protected_path("/etc/passwd"));
        assert!(config.is_protected_path("/boot/vmlinuz"));
        assert!(!config.is_protected_path("/home/user/file.txt"));
    }
}
