use anyhow::Result;
use clap::{Parser, Subcommand};

mod auth;
mod commands;

use commands::approval::ApprovalAction;

#[derive(Parser)]
#[command(name = "dots-family-ctl")]
#[command(about = "DOTS Family Mode CLI control tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },

    Session {
        #[command(subcommand)]
        action: SessionAction,
    },

    TimeWindow {
        #[command(subcommand)]
        action: TimeWindowAction,
    },

    Report {
        #[command(subcommand)]
        action: ReportAction,
    },

    Approval {
        #[command(subcommand)]
        action: ApprovalAction,
    },

    Status,

    Check {
        #[arg(help = "Application ID to check")]
        app_id: String,
    },
}

#[derive(Subcommand)]
enum ProfileAction {
    List,
    Show {
        name: String,
    },
    Create {
        name: String,
        age_group: String,
        #[arg(short, long, help = "System username for the profile")]
        username: Option<String>,
    },
    SetActive {
        profile_id: String,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    View,
    History { profile_id: Option<String> },
}

#[derive(Subcommand)]
enum TimeWindowAction {
    /// Add a time window to a profile
    Add {
        #[arg(help = "Profile name or ID")]
        profile: String,
        #[arg(long, help = "Add weekday time window")]
        weekday: bool,
        #[arg(long, help = "Add weekend time window")]
        weekend: bool,
        #[arg(long, help = "Add holiday time window")]
        holiday: bool,
        #[arg(help = "Start time (HH:MM format)")]
        start: String,
        #[arg(help = "End time (HH:MM format)")]
        end: String,
    },
    /// List time windows for a profile
    List {
        #[arg(help = "Profile name or ID")]
        profile: String,
    },
    /// Remove a time window from a profile
    Remove {
        #[arg(help = "Profile name or ID")]
        profile: String,
        #[arg(long, help = "Remove from weekday windows")]
        weekday: bool,
        #[arg(long, help = "Remove from weekend windows")]
        weekend: bool,
        #[arg(long, help = "Remove from holiday windows")]
        holiday: bool,
        #[arg(help = "Time window to remove (HH:MM-HH:MM format)")]
        window: String,
    },
    /// Clear all time windows for a profile
    Clear {
        #[arg(help = "Profile name or ID")]
        profile: String,
        #[arg(long, help = "Clear only weekday windows")]
        weekday: bool,
        #[arg(long, help = "Clear only weekend windows")]
        weekend: bool,
        #[arg(long, help = "Clear only holiday windows")]
        holiday: bool,
    },
}

#[derive(Subcommand)]
enum ReportAction {
    /// Get a daily activity report
    Daily {
        #[arg(help = "Profile name or ID")]
        profile: String,
        #[arg(short, long, help = "Date in YYYY-MM-DD format (defaults to today)")]
        date: Option<String>,
    },
    /// Get a weekly activity report
    Weekly {
        #[arg(help = "Profile name or ID")]
        profile: String,
        #[arg(
            short,
            long,
            help = "Week start date in YYYY-MM-DD format (defaults to current week Monday)"
        )]
        week_start: Option<String>,
    },
    /// Export reports to a file
    Export {
        #[arg(help = "Profile name or ID")]
        profile: String,
        #[arg(short, long, default_value = "json", help = "Export format (json or csv)")]
        format: String,
        #[arg(short, long, help = "Start date (YYYY-MM-DD)")]
        start_date: String,
        #[arg(short, long, help = "End date (YYYY-MM-DD)")]
        end_date: String,
        #[arg(short, long, help = "Output file path (prints to stdout if not specified)")]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Profile { action } => match action {
            ProfileAction::List => commands::profile::list().await?,
            ProfileAction::Show { name } => commands::profile::show(&name).await?,
            ProfileAction::Create { name, age_group, username } => {
                commands::profile::create(&name, &age_group, username.as_deref()).await?
            }
            ProfileAction::SetActive { profile_id } => {
                commands::profile::set_active(&profile_id).await?
            }
        },
        Commands::Session { action } => match action {
            SessionAction::View => commands::session::view().await?,
            SessionAction::History { profile_id } => {
                commands::session::history(profile_id.as_deref()).await?
            }
        },
        Commands::TimeWindow { action } => match action {
            TimeWindowAction::Add { profile, weekday, weekend, holiday, start, end } => {
                commands::time_window::add(&profile, weekday, weekend, holiday, &start, &end)
                    .await?
            }
            TimeWindowAction::List { profile } => commands::time_window::list(&profile).await?,
            TimeWindowAction::Remove { profile, weekday, weekend, holiday, window } => {
                commands::time_window::remove(&profile, weekday, weekend, holiday, &window).await?
            }
            TimeWindowAction::Clear { profile, weekday, weekend, holiday } => {
                commands::time_window::clear(&profile, weekday, weekend, holiday).await?
            }
        },
        Commands::Report { action } => match action {
            ReportAction::Daily { profile, date } => {
                commands::report::daily(&profile, date.as_deref()).await?
            }
            ReportAction::Weekly { profile, week_start } => {
                commands::report::weekly(&profile, week_start.as_deref()).await?
            }
            ReportAction::Export { profile, format, start_date, end_date, output } => {
                commands::report::export(
                    &profile,
                    &format,
                    &start_date,
                    &end_date,
                    output.as_deref(),
                )
                .await?
            }
        },
        Commands::Approval { action } => match action {
            ApprovalAction::List => commands::approval::list().await?,
            ApprovalAction::Approve { request_id, message } => {
                commands::approval::approve(request_id, message).await?
            }
            ApprovalAction::Deny { request_id, message } => {
                commands::approval::deny(request_id, message).await?
            }
        },
        Commands::Status => commands::status::show().await?,
        Commands::Check { app_id } => commands::check::application(&app_id).await?,
    }

    Ok(())
}
