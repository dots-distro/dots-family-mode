use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EducationalMessage {
    pub command: String,
    pub risk_level: String,
    pub explanation: String,
    pub safer_alternatives: Vec<String>,
    pub learning_resources: Vec<String>,
}

pub struct EducationalSystem {
    messages: HashMap<String, EducationalMessage>,
    safety_tips: Vec<String>,
}

impl EducationalSystem {
    pub fn new() -> Self {
        let mut system = Self { messages: HashMap::new(), safety_tips: Vec::new() };

        system.initialize_educational_content();
        system
    }

    fn initialize_educational_content(&mut self) {
        self.add_message(
            "rm -rf /",
            "critical",
            "This command attempts to delete EVERYTHING on your computer, including the operating system. This would make your computer completely unusable and destroy all your files permanently.",
            vec![
                "rm filename.txt (to delete a specific file)".to_string(),
                "rm -r folder/ (to delete a folder and its contents)".to_string(),
                "Move files to trash instead of permanent deletion".to_string(),
            ],
            vec![
                "Linux file management tutorial".to_string(),
                "Safe file deletion practices".to_string(),
            ],
        );

        self.add_message(
            "sudo",
            "high",
            "'sudo' gives you administrator privileges, allowing you to make system-wide changes. While necessary for some tasks, it can be dangerous if you're not sure what a command does.",
            vec![
                "Always read and understand commands before using sudo".to_string(),
                "Ask a parent or guardian before using sudo".to_string(),
                "Use regular commands without sudo when possible".to_string(),
            ],
            vec![
                "Understanding Linux permissions".to_string(),
                "When and why to use sudo".to_string(),
            ],
        );

        self.add_message(
            "curl.*\\|.*sh",
            "critical", 
            "This command downloads and immediately runs code from the internet without checking what it does first. This is extremely dangerous as it could install malware or damage your system.",
            vec![
                "Download the file first: curl -o script.sh http://example.com/script.sh".to_string(),
                "Read the script contents before running: cat script.sh".to_string(),
                "Ask for help understanding what scripts do".to_string(),
            ],
            vec![
                "Safe software installation practices".to_string(),
                "How to verify downloaded files".to_string(),
            ],
        );

        self.add_message(
            "dd",
            "critical",
            "The 'dd' command can overwrite entire disks and permanently destroy data. It's sometimes called 'disk destroyer' because of how dangerous it can be when used incorrectly.",
            vec![
                "Use file managers for copying files".to_string(),
                "Use 'cp' command for copying files safely".to_string(),
                "Always double-check device names before using dd".to_string(),
            ],
            vec![
                "Safe file copying methods".to_string(),
                "Understanding storage devices".to_string(),
            ],
        );

        self.add_message(
            "chmod 777",
            "high",
            "This command makes files readable, writable, and executable by everyone on the system. This removes all security protections and could allow malicious software to modify important files.",
            vec![
                "chmod 755 (makes files executable but keeps them secure)".to_string(),
                "chmod 644 (good for most regular files)".to_string(),
                "Ask what permissions are actually needed".to_string(),
            ],
            vec![
                "Linux file permissions explained".to_string(),
                "Security best practices".to_string(),
            ],
        );

        self.safety_tips = vec![
            "Always read error messages carefully - they often tell you exactly what's wrong"
                .to_string(),
            "When in doubt, ask for help rather than guessing with powerful commands".to_string(),
            "Test commands on unimportant files first before using them on important data"
                .to_string(),
            "Make backups of important files before making system changes".to_string(),
            "Use 'man' command to read the manual for any command (e.g., 'man ls')".to_string(),
            "Tab completion can help you avoid typos in file and command names".to_string(),
        ];
    }

    fn add_message(
        &mut self,
        command_pattern: &str,
        risk_level: &str,
        explanation: &str,
        alternatives: Vec<String>,
        resources: Vec<String>,
    ) {
        self.messages.insert(
            command_pattern.to_string(),
            EducationalMessage {
                command: command_pattern.to_string(),
                risk_level: risk_level.to_string(),
                explanation: explanation.to_string(),
                safer_alternatives: alternatives,
                learning_resources: resources,
            },
        );
    }

    pub fn get_educational_message(&self, command: &str, risk_level: &str) -> Option<String> {
        for (pattern, message) in &self.messages {
            if command.contains(&pattern.replace(".*", "")) {
                return Some(self.format_educational_message(message));
            }
        }

        self.get_generic_educational_message(risk_level)
    }

    fn format_educational_message(&self, message: &EducationalMessage) -> String {
        let mut output = String::new();

        output.push_str("🎓 LEARNING OPPORTUNITY\n");
        output.push_str(&format!("Command: {}\n", message.command));
        output.push_str(&format!("Risk Level: {}\n\n", message.risk_level.to_uppercase()));

        output.push_str("💡 What this command does:\n");
        output.push_str(&format!("{}\n\n", message.explanation));

        if !message.safer_alternatives.is_empty() {
            output.push_str("✅ Safer alternatives:\n");
            for alternative in &message.safer_alternatives {
                output.push_str(&format!("  • {}\n", alternative));
            }
            output.push('\n');
        }

        if !message.learning_resources.is_empty() {
            output.push_str("📚 Learn more about:\n");
            for resource in &message.learning_resources {
                output.push_str(&format!("  • {}\n", resource));
            }
            output.push('\n');
        }

        output.push_str("💬 Remember: It's always okay to ask for help when you're unsure!\n");

        output
    }

    fn get_generic_educational_message(&self, risk_level: &str) -> Option<String> {
        let message = match risk_level.to_lowercase().as_str() {
            "critical" => {
                "🚨 CRITICAL RISK DETECTED\n\
                This command could seriously damage your computer or compromise your safety.\n\
                Please ask a parent or guardian for help before proceeding.\n\n\
                💡 Tip: Commands that modify system files or download content from the internet often carry high risks."
            },
            "high" => {
                "⚠️  HIGH RISK COMMAND\n\
                This command requires administrator privileges or could affect system security.\n\
                Make sure you understand what it does and have permission to run it.\n\n\
                💡 Tip: Use 'man <command>' to read the manual for any command."
            },
            "medium" => {
                "⚡ MODERATE RISK\n\
                This command might modify files or system settings.\n\
                Double-check the command and make sure it's what you intend to do.\n\n\
                💡 Tip: Test commands on unimportant files first when learning."
            },
            _ => return None,
        };

        Some(format!("{}\n\n{}", message, self.get_random_safety_tip()))
    }

    fn get_random_safety_tip(&self) -> String {
        if self.safety_tips.is_empty() {
            return String::new();
        }

        let index = fastrand::usize(..self.safety_tips.len());
        format!("🧠 Safety tip: {}", self.safety_tips[index])
    }

    pub fn get_command_blocked_message(&self, command: &str, reasons: &[String]) -> String {
        let mut message = "🛑 COMMAND BLOCKED FOR SAFETY\n\n".to_string();
        message.push_str(&format!("Command: {}\n\n", command));

        if !reasons.is_empty() {
            message.push_str("🔍 Why this command was blocked:\n");
            for reason in reasons {
                message.push_str(&format!("  • {}\n", reason));
            }
            message.push('\n');
        }

        if let Some(educational) = self.get_educational_message(command, "high") {
            message.push_str(&educational);
        } else {
            message.push_str("💬 If you believe this command should be allowed, please ask a parent or guardian.\n");
            message.push_str("They can temporarily disable filtering or add this command to the allowed list.\n\n");
            message.push_str(&self.get_random_safety_tip());
        }

        message
    }

    pub fn get_approval_required_message(&self, command: &str, reasons: &[String]) -> String {
        let mut message = "🔐 PARENT APPROVAL REQUIRED\n\n".to_string();
        message.push_str(&format!("Command: {}\n\n", command));

        message.push_str("This command requires permission because:\n");
        for reason in reasons {
            message.push_str(&format!("  • {}\n", reason));
        }
        message.push('\n');

        if let Some(educational) = self.get_educational_message(command, "medium") {
            message.push_str(&educational);
        }

        message.push_str("\n📞 An approval request has been sent to your parent/guardian.\n");
        message.push_str("You'll be notified once they respond.\n");

        message
    }
}

impl Default for EducationalSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_educational_system_creation() {
        let system = EducationalSystem::new();
        assert!(!system.messages.is_empty());
        assert!(!system.safety_tips.is_empty());
    }

    #[test]
    fn test_dangerous_command_messages() {
        let system = EducationalSystem::new();

        let message = system.get_educational_message("rm -rf /", "critical");
        assert!(message.is_some());
        let content = message.unwrap();
        assert!(content.contains("LEARNING OPPORTUNITY"));
        assert!(content.contains("delete EVERYTHING"));
    }

    #[test]
    fn test_blocked_message_format() {
        let system = EducationalSystem::new();
        let message = system
            .get_command_blocked_message("sudo rm -rf /", &["Extremely dangerous".to_string()]);

        assert!(message.contains("COMMAND BLOCKED"));
        assert!(message.contains("sudo rm -rf /"));
        assert!(message.contains("Extremely dangerous"));
    }

    #[test]
    fn test_approval_message_format() {
        let system = EducationalSystem::new();
        let message = system
            .get_approval_required_message("sudo apt install", &["Requires admin".to_string()]);

        assert!(message.contains("PARENT APPROVAL REQUIRED"));
        assert!(message.contains("sudo apt install"));
        assert!(message.contains("Requires admin"));
    }

    #[test]
    fn test_generic_risk_messages() {
        let system = EducationalSystem::new();

        assert!(system.get_educational_message("unknown_command", "critical").is_some());
        assert!(system.get_educational_message("unknown_command", "high").is_some());
        assert!(system.get_educational_message("unknown_command", "medium").is_some());
        assert!(system.get_educational_message("unknown_command", "safe").is_none());
    }
}
