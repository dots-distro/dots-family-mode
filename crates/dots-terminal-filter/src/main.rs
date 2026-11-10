use anyhow::{Context, Result};
use clap::{Arg, ArgMatches, Command};
use dots_family_proto::daemon::FamilyDaemonProxy;
use tracing::{debug, info, warn};
use zbus::Connection;

mod command_parser;
mod config;
mod educational;
mod policy;
mod risk_analyzer;
mod shell_wrapper;

use config::TerminalConfig;
use risk_analyzer::RiskAnalyzer;
use shell_wrapper::ShellWrapper;

/// Exit code when the command is allowed, including a command that is only
/// warned about.
const EXIT_ALLOWED: i32 = 0;
/// Exit code when local policy, risk analysis or the daemon denies the command.
const EXIT_DENIED: i32 = 1;
/// Exit code when the daemon policy cannot be obtained, so the answer is
/// unknown. It is deliberately distinct from both allow and deny.
const EXIT_UNDETERMINABLE: i32 = 3;

/// The three answers `--check-only` can give. There is no fourth "probably
/// allowed": an unreachable daemon is `Undeterminable`, never `Allowed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckOutcome {
    Allowed,
    Denied,
    Undeterminable,
}

struct CheckResult {
    outcome: CheckOutcome,
    reason: Option<String>,
}

fn exit_code_for(outcome: CheckOutcome) -> i32 {
    match outcome {
        CheckOutcome::Allowed => EXIT_ALLOWED,
        CheckOutcome::Denied => EXIT_DENIED,
        CheckOutcome::Undeterminable => EXIT_UNDETERMINABLE,
    }
}

/// Ask the daemon the same question the wrapper asks and reduce it with the
/// shared `policy::profile_verdict`, so check-only has no separate decision.
fn classify_policy_verdict(command: &str, answer: Option<zbus::Result<bool>>) -> CheckResult {
    match crate::policy::profile_verdict(command, answer) {
        Ok(true) => CheckResult { outcome: CheckOutcome::Allowed, reason: None },
        Ok(false) => CheckResult {
            outcome: CheckOutcome::Denied,
            reason: Some("blocked by the active profile".to_string()),
        },
        Err(e) => {
            CheckResult { outcome: CheckOutcome::Undeterminable, reason: Some(e.to_string()) }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("dots-terminal-filter")
        .version("0.1.0")
        .about("DOTS Family Mode Terminal Filter")
        .arg(
            Arg::new("check-only")
                .long("check-only")
                .action(clap::ArgAction::SetTrue)
                .help("Only check command safety, don't execute"),
        )
        .arg(
            Arg::new("shell")
                .long("shell")
                .value_name("SHELL")
                .help("Shell type (bash, zsh, fish)"),
        )
        .arg(
            Arg::new("command")
                .long("command")
                .value_name("COMMAND")
                .help("Command to check or execute"),
        )
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .action(clap::ArgAction::SetTrue)
                .help("Run in interactive mode"),
        )
        .arg(Arg::new("args").action(clap::ArgAction::Append).help("Command arguments"))
        .after_help(
            "Exit codes:\n  \
             0  allowed (including a warned command)\n  \
             1  denied\n  \
             3  undeterminable: the daemon policy could not be obtained, so the command is \
             not reported as allowed and the wrapper will deny it",
        )
        .get_matches();

    setup_logging(&matches)?;

    let config = TerminalConfig::load().context("Failed to load configuration")?;
    let risk_analyzer = RiskAnalyzer::new(&config);

    if matches.get_flag("check-only") {
        handle_check_only_mode(&matches, &config, &risk_analyzer).await
    } else if matches.get_flag("interactive") {
        let mut shell_wrapper = ShellWrapper::new(config, risk_analyzer).await?;
        shell_wrapper.run_interactive().await
    } else if let Some(command) = matches.get_one::<String>("command") {
        let args: Vec<String> = vec![command.clone()];
        let mut shell_wrapper = ShellWrapper::new(config, risk_analyzer).await?;
        shell_wrapper.execute_command(&args).await
    } else if let Some(args) = matches.get_many::<String>("args") {
        let args: Vec<String> = args.cloned().collect();
        let mut shell_wrapper = ShellWrapper::new(config, risk_analyzer).await?;
        shell_wrapper.execute_command(&args).await
    } else {
        let mut shell_wrapper = ShellWrapper::new(config, risk_analyzer).await?;
        shell_wrapper.run_interactive().await
    }
}

fn setup_logging(matches: &ArgMatches) -> Result<()> {
    let log_level = if matches.get_flag("check-only") { "warn" } else { "info" };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    if !matches.get_flag("check-only") {
        info!("Starting DOTS Terminal Filter");
    }

    Ok(())
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

/// Ask the daemon the profile question for a command and reduce the answer with
/// the same function the wrapper uses.
async fn check_daemon_policy(interface: &str, command: &str) -> CheckResult {
    let answer = match connect_to_daemon(interface).await {
        Some(proxy) => {
            let app_name = command.split_whitespace().next().unwrap_or("unknown");
            Some(proxy.check_application_allowed(app_name).await)
        }
        None => None,
    };

    classify_policy_verdict(command, answer)
}

async fn handle_check_only_mode(
    matches: &ArgMatches,
    config: &TerminalConfig,
    risk_analyzer: &RiskAnalyzer,
) -> Result<()> {
    let command =
        matches.get_one::<String>("command").context("Command required in check-only mode")?;

    let _shell = matches.get_one::<String>("shell").map(|s| s.as_str()).unwrap_or("bash");

    if !config.enabled {
        std::process::exit(EXIT_ALLOWED);
    }

    if config.is_blocked_command(command) {
        eprintln!("Command blocked by configuration: {}", command);
        std::process::exit(EXIT_DENIED);
    }

    if config.is_allowed_command(command) {
        std::process::exit(EXIT_ALLOWED);
    }

    let assessment = risk_analyzer.analyze_command(command);

    match assessment.recommended_action {
        crate::risk_analyzer::RecommendedAction::Allow => {}
        crate::risk_analyzer::RecommendedAction::Warn => {
            eprintln!("Warning: Command flagged as potentially risky");
            for reason in &assessment.reasons {
                eprintln!("  - {}", reason);
            }
        }
        crate::risk_analyzer::RecommendedAction::Block => {
            eprintln!("Command blocked due to high risk: {}", command);
            for reason in &assessment.reasons {
                eprintln!("  - {}", reason);
            }
            std::process::exit(EXIT_DENIED);
        }
        crate::risk_analyzer::RecommendedAction::RequireApproval => {
            eprintln!("Command requires parent approval: {}", command);
            for reason in &assessment.reasons {
                eprintln!("  - {}", reason);
            }
            std::process::exit(EXIT_DENIED);
        }
    }

    // The wrapper asks the daemon whenever local policy allowed the command.
    // Check-only must ask the same question instead of returning its local
    // allow verdict.
    if config.check_profile_restrictions {
        let result = check_daemon_policy(&config.daemon_interface, command).await;

        match result.outcome {
            CheckOutcome::Allowed => {}
            CheckOutcome::Denied => {
                eprintln!("Command blocked by active profile: {}", command);
                std::process::exit(exit_code_for(result.outcome));
            }
            CheckOutcome::Undeterminable => {
                eprintln!(
                    "Unknown: {}",
                    result.reason.as_deref().unwrap_or("daemon policy unavailable")
                );
                eprintln!(
                    "Command '{}' is not reported as allowed; the wrapper will deny it.",
                    command
                );
                std::process::exit(exit_code_for(result.outcome));
            }
        }
    }

    std::process::exit(EXIT_ALLOWED);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_only_reports_denied_when_the_daemon_policy_denies() {
        let result = classify_policy_verdict("curl http://example.com", Some(Ok(false)));

        assert_eq!(result.outcome, CheckOutcome::Denied);
        assert_ne!(result.outcome, CheckOutcome::Allowed);
        assert_eq!(exit_code_for(result.outcome), EXIT_DENIED);
    }

    #[test]
    fn check_only_does_not_report_allowed_when_the_daemon_is_unreachable() {
        let result = classify_policy_verdict("curl http://example.com", None);

        assert_ne!(result.outcome, CheckOutcome::Allowed);
        assert_eq!(result.outcome, CheckOutcome::Undeterminable);
        assert_eq!(exit_code_for(result.outcome), EXIT_UNDETERMINABLE);
        assert_ne!(EXIT_UNDETERMINABLE, EXIT_ALLOWED);
        assert_ne!(EXIT_UNDETERMINABLE, EXIT_DENIED);
        assert!(result
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("Policy could not be checked"));
    }
}
