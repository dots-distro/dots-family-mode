use anyhow::Result;
use clap::{Parser, Subcommand};

mod auth;
mod commands;

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
        Commands::Status => commands::status::show().await?,
        Commands::Check { app_id } => commands::check::application(&app_id).await?,
    }

    Ok(())
}
