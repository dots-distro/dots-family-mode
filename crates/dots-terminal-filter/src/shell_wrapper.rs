use anyhow::{Context, Result};
use dots_family_common::types::{Activity, ActivityType};
use dots_family_proto::daemon::FamilyDaemonProxy;
use std::process::{Command, Stdio};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};
use zbus::Connection;

use crate::config::TerminalConfig;
use crate::educational::EducationalSystem;
use crate::risk_analyzer::{RecommendedAction, RiskAnalyzer, RiskLevel};

pub struct ShellWrapper {
    config: TerminalConfig,
    risk_analyzer: RiskAnalyzer,
    educational_system: EducationalSystem,
    daemon_proxy: Option<FamilyDaemonProxy<'static>>,
}

impl ShellWrapper {
    pub async fn new(config: TerminalConfig, risk_analyzer: RiskAnalyzer) -> Result<Self> {
        let daemon_proxy = if config.check_profile_restrictions {
            Self::connect_to_daemon(&config.daemon_interface).await
        } else {
            None
        };

        Ok(Self {
            config,
            risk_analyzer,
            educational_system: EducationalSystem::new(),
            daemon_proxy,
        })
    }

    async fn connect_to_daemon(interface: &str) -> Option<FamilyDaemonProxy<'static>> {
        match Connection::system().await {
            Ok(conn) => match FamilyDaemonProxy::new(&conn).await {
                Ok(proxy) => {
                    debug!("Connected to daemon via DBus: {}", interface);
                    Some(proxy)
                }
                Err(e) => {
                    warn!("Failed to connect to daemon: {}. Running in standalone mode.", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to connect to system bus: {}. Running in standalone mode.", e);
                None
            }
        }
    }

    pub async fn execute_command(&mut self, args: &[String]) -> Result<()> {
        let command_line = args.join(" ");
        info!("Executing command: {}", command_line);

        match self.evaluate_command(&command_line).await {
            Ok(true) => self.run_command(args).await,
            Ok(false) => {
                eprintln!("Command blocked by DOTS Family Mode");
                std::process::exit(1);
            }
            Err(e) => {
                error!("Error evaluating command: {}", e);
                eprintln!("Error evaluating command: {}", e);
                std::process::exit(1);
            }
        }
    }

    pub async fn run_interactive(&mut self) -> Result<()> {
        println!("DOTS Family Mode Terminal Filter");
        println!("Type 'exit' to quit, 'help' for assistance");

        let mut stdin = BufReader::new(io::stdin());
        let mut stdout = io::stdout();

        loop {
            print!("dots-shell> ");
            stdout.flush().await?;

            let mut input = String::new();
            match stdin.read_line(&mut input).await {
                Ok(0) => break,
                Ok(_) => {
                    let command = input.trim();

                    if command.is_empty() {
                        continue;
                    }

                    if command == "exit" {
                        break;
                    }

                    if command == "help" {
                        self.show_help().await;
                        continue;
                    }

                    let args: Vec<String> = shell_words(command);

                    match self.evaluate_command(command).await {
                        Ok(true) => {
                            if let Err(e) = self.run_command(&args).await {
                                eprintln!("Error executing command: {}", e);
                            }
                        }
                        Ok(false) => {
                            eprintln!("Command blocked by DOTS Family Mode");
                        }
                        Err(e) => {
                            error!("Error evaluating command: {}", e);
                            eprintln!("Error evaluating command: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading input: {}", e);
                    break;
                }
            }
        }

        println!("Goodbye!");
        Ok(())
    }

    async fn evaluate_command(&mut self, command: &str) -> Result<bool> {
        if !self.config.enabled {
            return Ok(true);
        }

        debug!("Evaluating command: {}", command);

        if self.config.is_blocked_command(command) {
            warn!("Command blocked by configuration: {}", command);
            return Ok(false);
        }

        if self.config.is_allowed_command(command) {
            debug!("Command explicitly allowed: {}", command);
            self.log_command_activity(command, "allowed").await;
            return Ok(true);
        }

        let assessment = self.risk_analyzer.analyze_command(command);

        debug!("Risk assessment: {:?}", assessment);

        if self.config.shell.show_warnings && assessment.risk_level >= RiskLevel::Medium {
            self.show_risk_warning(&assessment).await;
        }

        let allowed = match assessment.recommended_action {
            RecommendedAction::Allow => {
                self.log_command_activity(command, "allowed").await;
                true
            }
            RecommendedAction::Warn => {
                self.log_command_activity(command, "warned").await;
                true
            }
            RecommendedAction::Block => {
                warn!("Command blocked due to high risk: {}", command);
                let blocked_msg = self
                    .educational_system
                    .get_command_blocked_message(command, &assessment.reasons);
                eprintln!("{}", blocked_msg);
                self.log_command_activity(command, "blocked").await;
                false
            }
            RecommendedAction::RequireApproval => {
                self.request_parent_approval(command, &assessment).await
            }
        };

        if self.config.check_profile_restrictions && allowed {
            return self.check_profile_restrictions(command).await;
        }

        Ok(allowed)
    }

    async fn run_command(&self, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Ok(());
        }

        let mut cmd = Command::new(&args[0]);
        if args.len() > 1 {
            cmd.args(&args[1..]);
        }

        let status = cmd
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Failed to execute command")?;

        if !status.success() {
            if let Some(code) = status.code() {
                debug!("Command exited with code: {}", code);
                std::process::exit(code);
            }
        }

        Ok(())
    }

    async fn show_help(&self) {
        println!("DOTS Family Mode Terminal Filter Help");
        println!();
        println!("Available commands:");
        println!("  help    - Show this help message");
        println!("  exit    - Exit the terminal filter");
        println!();
        println!("All other commands are filtered based on:");
        println!("  - Risk analysis");
        println!("  - Profile restrictions");
        println!("  - Parent approval requirements");
        println!();
        println!("If a command is blocked, contact your parent or guardian.");
    }

    async fn show_risk_warning(&self, assessment: &crate::risk_analyzer::RiskAssessment) {
        let risk_level = format!("{:?}", assessment.risk_level);
        if let Some(educational_msg) =
            self.educational_system.get_educational_message(&assessment.command, &risk_level)
        {
            eprintln!("{}", educational_msg);
        } else {
            eprintln!("⚠️  WARNING: This command has been flagged as potentially risky");
            eprintln!("   Risk Level: {:?}", assessment.risk_level);
            eprintln!("   Reasons:");
            for reason in &assessment.reasons {
                eprintln!("     • {}", reason);
            }
            eprintln!();
        }
    }

    async fn request_parent_approval(
        &mut self,
        command: &str,
        assessment: &crate::risk_analyzer::RiskAssessment,
    ) -> bool {
        let approval_msg =
            self.educational_system.get_approval_required_message(command, &assessment.reasons);
        println!("{}", approval_msg);

        if let Some(ref proxy) = self.daemon_proxy {
            let risk_level = format!("{:?}", assessment.risk_level);
            let reasons = assessment.reasons.join(", ");

            match proxy.request_command_approval(command, &risk_level, &reasons).await {
                Ok(response) => match serde_json::from_str::<serde_json::Value>(&response) {
                    Ok(json) => {
                        if let Some(approval_id) = json.get("approval_id").and_then(|v| v.as_str())
                        {
                            println!("   ✓ Approval request submitted (ID: {})", approval_id);
                            if let Some(message) = json.get("message").and_then(|v| v.as_str()) {
                                println!("   Message: {}", message);
                            }
                            self.log_command_activity(command, "approval_requested").await;
                            return false;
                        } else if json.get("error").is_some() {
                            println!(
                                "   ✗ Approval request failed: {}",
                                json.get("error").unwrap_or(&serde_json::Value::Null)
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse approval response: {}", e);
                        println!("   ✗ Approval system error");
                    }
                },
                Err(e) => {
                    warn!("Failed to request command approval: {}", e);
                    println!("   ✗ Unable to connect to approval system");
                }
            }
        } else {
            println!("   ✗ Approval system not available (daemon not connected)");
        }

        println!("   For safety, this command will be blocked.");
        self.log_command_activity(command, "blocked_needs_approval").await;
        false
    }

    async fn check_profile_restrictions(&self, command: &str) -> Result<bool> {
        if let Some(ref proxy) = self.daemon_proxy {
            let cmd_parts: Vec<&str> = command.split_whitespace().collect();
            let app_name = cmd_parts.first().unwrap_or(&"unknown");

            match proxy.check_application_allowed(app_name).await {
                Ok(allowed) => {
                    if !allowed {
                        warn!("Command '{}' blocked by active profile", command);
                    }
                    Ok(allowed)
                }
                Err(e) => {
                    warn!("Failed to check profile restrictions: {}", e);
                    Ok(true)
                }
            }
        } else {
            Ok(true)
        }
    }

    async fn log_command_activity(&self, command: &str, action: &str) {
        if !self.config.log_all_commands {
            return;
        }

        if let Some(ref proxy) = self.daemon_proxy {
            let activity = Activity {
                id: uuid::Uuid::new_v4(),
                profile_id: uuid::Uuid::nil(),
                timestamp: chrono::Utc::now(),
                activity_type: ActivityType::TerminalCommand { command: command.to_string() },
                application: Some("terminal-filter".to_string()),
                window_title: Some(format!("Terminal: {} - {}", action, command)),
                duration_seconds: 0,
            };

            let activity_json = serde_json::to_string(&activity).unwrap_or_default();
            if let Err(e) = proxy.report_activity(&activity_json).await {
                warn!("Failed to log command activity: {}", e);
            }
        }
    }
}

pub fn shell_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current_word = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in input.chars() {
        match ch {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
            }
            ch if in_quotes && ch == quote_char => {
                in_quotes = false;
            }
            ' ' | '\t' if !in_quotes => {
                if !current_word.is_empty() {
                    words.push(current_word.clone());
                    current_word.clear();
                }
            }
            _ => {
                current_word.push(ch);
            }
        }
    }

    if !current_word.is_empty() {
        words.push(current_word);
    }

    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_words() {
        assert_eq!(shell_words("ls -la"), vec!["ls", "-la"]);
        assert_eq!(shell_words("echo 'hello world'"), vec!["echo", "hello world"]);
        assert_eq!(
            shell_words("grep \"test string\" file.txt"),
            vec!["grep", "test string", "file.txt"]
        );
        assert_eq!(shell_words(""), Vec::<String>::new());
    }

    #[tokio::test]
    async fn test_shell_wrapper_creation() {
        let config = TerminalConfig::default();
        let risk_analyzer = RiskAnalyzer::new(&config);
        let _wrapper = ShellWrapper::new(config, risk_analyzer).await.unwrap();
    }
}
