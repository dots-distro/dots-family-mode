use regex::Regex;
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::config::{RiskAnalysisConfig, TerminalConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "safe" => Self::Safe,
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" => Self::High,
            "critical" => Self::Critical,
            _ => Self::Medium,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub risk_level: RiskLevel,
    pub reasons: Vec<String>,
    pub command: String,
    pub recommended_action: RecommendedAction,
}

#[derive(Debug, Clone)]
pub enum RecommendedAction {
    Allow,
    Warn,
    Block,
    RequireApproval,
}

pub struct RiskAnalyzer {
    config: RiskAnalysisConfig,
    high_risk_patterns: Vec<Regex>,
    command_risk_map: HashMap<String, RiskLevel>,
}

impl RiskAnalyzer {
    pub fn new(config: &TerminalConfig) -> Self {
        let mut analyzer = Self {
            config: config.risk_analysis.clone(),
            high_risk_patterns: Vec::new(),
            command_risk_map: HashMap::new(),
        };

        analyzer.initialize_patterns();
        analyzer.initialize_command_risks();
        analyzer
    }

    fn initialize_patterns(&mut self) {
        for pattern_str in &self.config.high_risk_patterns {
            match Regex::new(pattern_str) {
                Ok(regex) => self.high_risk_patterns.push(regex),
                Err(e) => warn!("Invalid regex pattern '{}': {}", pattern_str, e),
            }
        }
    }

    fn initialize_command_risks(&mut self) {
        // Critical risk commands
        let critical_commands = [
            "rm -rf /", "dd", "mkfs", "fdisk", "parted", "cfdisk", "format", "shred", "wipefs",
            "sgdisk",
        ];
        for cmd in &critical_commands {
            self.command_risk_map.insert(cmd.to_string(), RiskLevel::Critical);
        }

        // High risk commands
        let high_risk_commands = [
            "sudo",
            "su",
            "passwd",
            "userdel",
            "usermod",
            "groupdel",
            "groupmod",
            "systemctl",
            "service",
            "mount",
            "umount",
            "chmod 777",
            "chown",
            "iptables",
            "ufw",
            "firewall-cmd",
            "modprobe",
            "rmmod",
            "insmod",
            "kill -9",
            "killall",
            "pkill",
            "reboot",
            "shutdown",
            "halt",
        ];
        for cmd in &high_risk_commands {
            self.command_risk_map.insert(cmd.to_string(), RiskLevel::High);
        }

        // Medium risk commands
        let medium_risk_commands = [
            "chmod",
            "git clone",
            "curl",
            "wget",
            "ssh",
            "scp",
            "rsync",
            "tar",
            "unzip",
            "gunzip",
            "bunzip2",
            "pip install",
            "npm install",
            "make install",
            "dpkg",
            "apt install",
            "yum install",
            "dnf install",
        ];
        for cmd in &medium_risk_commands {
            self.command_risk_map.insert(cmd.to_string(), RiskLevel::Medium);
        }

        // Low risk commands
        let low_risk_commands = [
            "ps", "top", "htop", "free", "df", "du", "lscpu", "lsblk", "netstat", "ss", "lsof",
            "who", "w", "last", "history", "env", "printenv", "set", "alias", "type", "which",
            "whereis",
        ];
        for cmd in &low_risk_commands {
            self.command_risk_map.insert(cmd.to_string(), RiskLevel::Low);
        }
    }

    pub fn analyze_command(&self, command: &str) -> RiskAssessment {
        if !self.config.enabled {
            return RiskAssessment {
                risk_level: RiskLevel::Safe,
                reasons: vec!["Risk analysis disabled".to_string()],
                command: command.to_string(),
                recommended_action: RecommendedAction::Allow,
            };
        }

        let mut risk_level = RiskLevel::Safe;
        let mut reasons = Vec::new();

        debug!("Analyzing command: {}", command);

        // Check command length
        if command.len() > self.config.max_command_length {
            risk_level = RiskLevel::Medium;
            reasons.push(format!(
                "Command length ({}) exceeds maximum ({})",
                command.len(),
                self.config.max_command_length
            ));
        }

        // Check high-risk patterns
        for pattern in &self.high_risk_patterns {
            if pattern.is_match(command) {
                risk_level = std::cmp::max(risk_level, RiskLevel::Critical);
                reasons.push(format!("Matches high-risk pattern: {}", pattern.as_str()));
            }
        }

        // Analyze individual command components
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if !parts.is_empty() {
            let main_command = parts[0];

            // Check direct command mapping
            if let Some(&cmd_risk) = self.command_risk_map.get(main_command) {
                risk_level = std::cmp::max(risk_level, cmd_risk);
                reasons.push(format!(
                    "Command '{}' has {} risk",
                    main_command,
                    match cmd_risk {
                        RiskLevel::Safe => "safe",
                        RiskLevel::Low => "low",
                        RiskLevel::Medium => "medium",
                        RiskLevel::High => "high",
                        RiskLevel::Critical => "critical",
                    }
                ));
            }

            // Check command + arguments combinations
            if parts.len() > 1 {
                let full_cmd = format!("{} {}", parts[0], parts[1]);
                if let Some(&cmd_risk) = self.command_risk_map.get(&full_cmd) {
                    risk_level = std::cmp::max(risk_level, cmd_risk);
                    reasons.push(format!(
                        "Command combination '{}' has {} risk",
                        full_cmd,
                        match cmd_risk {
                            RiskLevel::Safe => "safe",
                            RiskLevel::Low => "low",
                            RiskLevel::Medium => "medium",
                            RiskLevel::High => "high",
                            RiskLevel::Critical => "critical",
                        }
                    ));
                }
            }
        }

        // Additional heuristics
        self.analyze_heuristics(command, &mut risk_level, &mut reasons);

        // Determine recommended action
        let recommended_action = self.determine_action(&risk_level);

        RiskAssessment { risk_level, reasons, command: command.to_string(), recommended_action }
    }

    fn analyze_heuristics(
        &self,
        command: &str,
        risk_level: &mut RiskLevel,
        reasons: &mut Vec<String>,
    ) {
        // Check for pipe to shell patterns
        if command.contains("| sh") || command.contains("| bash") || command.contains("| zsh") {
            *risk_level = std::cmp::max(*risk_level, RiskLevel::High);
            reasons.push("Contains pipe to shell execution".to_string());
        }

        // Check for suspicious URLs
        if (command.contains("curl") || command.contains("wget"))
            && (command.contains("http://") || command.contains("https://"))
        {
            *risk_level = std::cmp::max(*risk_level, RiskLevel::Medium);
            reasons.push("Downloads content from internet".to_string());
        }

        // Check for privilege escalation
        if command.contains("sudo") && (command.contains("-s") || command.contains("-i")) {
            *risk_level = std::cmp::max(*risk_level, RiskLevel::High);
            reasons.push("Attempts privilege escalation to shell".to_string());
        }

        // Check for mass file operations
        if command.contains("*")
            && (command.starts_with("rm")
                || command.starts_with("chmod")
                || command.starts_with("chown"))
        {
            *risk_level = std::cmp::max(*risk_level, RiskLevel::Medium);
            reasons.push("Mass file operation with wildcards".to_string());
        }

        // Check for suspicious file paths
        let suspicious_paths = ["/etc", "/boot", "/sys", "/proc", "/dev"];
        for path in &suspicious_paths {
            if command.contains(path)
                && (command.contains("rm") || command.contains("mv") || command.contains(">"))
            {
                *risk_level = std::cmp::max(*risk_level, RiskLevel::High);
                reasons.push(format!("Modifies system path: {}", path));
            }
        }

        // Check for command substitution
        if command.contains("$(") || command.contains("`") {
            *risk_level = std::cmp::max(*risk_level, RiskLevel::Medium);
            reasons.push("Contains command substitution".to_string());
        }

        // Check for redirection to devices
        if command.contains("> /dev/") {
            *risk_level = std::cmp::max(*risk_level, RiskLevel::High);
            reasons.push("Redirects output to device file".to_string());
        }
    }

    fn determine_action(&self, risk_level: &RiskLevel) -> RecommendedAction {
        let block_threshold = RiskLevel::from_string(&self.config.block_threshold);
        let approval_threshold = RiskLevel::from_string(&self.config.approval_threshold);

        if *risk_level >= block_threshold {
            RecommendedAction::Block
        } else if *risk_level >= approval_threshold {
            RecommendedAction::RequireApproval
        } else if *risk_level >= RiskLevel::Medium {
            RecommendedAction::Warn
        } else {
            RecommendedAction::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TerminalConfig;

    #[test]
    fn test_safe_commands() {
        let config = TerminalConfig::default();
        let analyzer = RiskAnalyzer::new(&config);

        let assessment = analyzer.analyze_command("ls -la");
        assert_eq!(assessment.risk_level, RiskLevel::Safe);
        assert!(matches!(assessment.recommended_action, RecommendedAction::Allow));

        let assessment = analyzer.analyze_command("cat file.txt");
        assert_eq!(assessment.risk_level, RiskLevel::Safe);
    }

    #[test]
    fn test_high_risk_commands() {
        let config = TerminalConfig::default();
        let analyzer = RiskAnalyzer::new(&config);

        let assessment = analyzer.analyze_command("sudo rm -rf /");
        assert!(assessment.risk_level >= RiskLevel::Critical);
        assert!(matches!(assessment.recommended_action, RecommendedAction::Block));

        let assessment = analyzer.analyze_command("dd if=/dev/zero of=/dev/sda");
        assert!(assessment.risk_level >= RiskLevel::Critical);
    }

    #[test]
    fn test_medium_risk_commands() {
        let config = TerminalConfig::default();
        let analyzer = RiskAnalyzer::new(&config);

        let assessment = analyzer.analyze_command("curl http://example.com/script.sh | bash");
        assert!(assessment.risk_level >= RiskLevel::High);

        let assessment = analyzer.analyze_command("chmod 755 script.sh");
        assert!(assessment.risk_level >= RiskLevel::Medium);
    }

    #[test]
    fn test_heuristics() {
        let config = TerminalConfig::default();
        let analyzer = RiskAnalyzer::new(&config);

        // Test pipe to shell detection
        let assessment = analyzer.analyze_command("wget -O - http://example.com/install.sh | sh");
        assert!(assessment.risk_level >= RiskLevel::High);
        assert!(assessment.reasons.iter().any(|r| r.contains("pipe to shell")));

        // Test system path modification
        let assessment = analyzer.analyze_command("rm /etc/passwd");
        assert!(assessment.risk_level >= RiskLevel::High);
        assert!(assessment.reasons.iter().any(|r| r.contains("system path")));
    }
}
