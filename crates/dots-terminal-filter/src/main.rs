use anyhow::{Context, Result};
use clap::{Arg, ArgMatches, Command};
use tracing::info;

mod command_parser;
mod config;
mod educational;
mod risk_analyzer;
mod shell_wrapper;

use config::TerminalConfig;
use risk_analyzer::RiskAnalyzer;
use shell_wrapper::ShellWrapper;

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

async fn handle_check_only_mode(
    matches: &ArgMatches,
    config: &TerminalConfig,
    risk_analyzer: &RiskAnalyzer,
) -> Result<()> {
    let command =
        matches.get_one::<String>("command").context("Command required in check-only mode")?;

    let _shell = matches.get_one::<String>("shell").map(|s| s.as_str()).unwrap_or("bash");

    if !config.enabled {
        std::process::exit(0);
    }

    if config.is_blocked_command(command) {
        eprintln!("Command blocked by configuration: {}", command);
        std::process::exit(1);
    }

    if config.is_allowed_command(command) {
        std::process::exit(0);
    }

    let assessment = risk_analyzer.analyze_command(command);

    match assessment.recommended_action {
        crate::risk_analyzer::RecommendedAction::Allow => {
            std::process::exit(0);
        }
        crate::risk_analyzer::RecommendedAction::Warn => {
            eprintln!("Warning: Command flagged as potentially risky");
            for reason in &assessment.reasons {
                eprintln!("  - {}", reason);
            }
            std::process::exit(0);
        }
        crate::risk_analyzer::RecommendedAction::Block => {
            eprintln!("Command blocked due to high risk: {}", command);
            for reason in &assessment.reasons {
                eprintln!("  - {}", reason);
            }
            std::process::exit(1);
        }
        crate::risk_analyzer::RecommendedAction::RequireApproval => {
            eprintln!("Command requires parent approval: {}", command);
            for reason in &assessment.reasons {
                eprintln!("  - {}", reason);
            }
            std::process::exit(1);
        }
    }
}
