mod commands;
mod human;
mod logging;
mod ui;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

#[derive(Parser)]
#[command(name = "linkd", version, about = "Local-dev link daemon for npm/pnpm")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Link a local package source into a consumer project
    Link {
        source: std::path::PathBuf,
        #[arg(default_value = ".")]
        consumer: std::path::PathBuf,
        #[arg(long)]
        copy: bool,
        #[arg(long)]
        hardlink: bool,
        #[arg(long)]
        link: bool,
    },
    /// Remove an active link
    Unlink {
        target: String,
    },
    /// List active links
    List,
    /// Run daemon in foreground with live terminal UI
    Watch,
    /// One-shot status snapshot
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Environment diagnostics
    Doctor {
        #[arg(long)]
        explain: Option<String>,
    },
    /// View daemon logs
    Logs {
        #[arg(long, short = 'f')]
        follow: bool,
    },
    /// Interactive first-time setup wizard
    Init,
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Link {
            source,
            consumer,
            copy,
            hardlink,
            link,
        } => commands::link::run(source, consumer, copy, hardlink, link).await,
        Commands::Unlink { target } => commands::unlink::run(&target).await,
        Commands::List => commands::list::run().await,
        Commands::Watch => commands::watch::run().await,
        Commands::Status { json } => commands::status::run(json).await,
        Commands::Doctor { explain } => commands::doctor::run(explain.as_deref()).await,
        Commands::Logs { follow } => commands::logs::run(follow).await,
        Commands::Init => commands::init::run().await,
        Commands::Completions { shell } => {
            generate(shell, &mut Cli::command(), "linkd", &mut std::io::stdout());
            Ok(())
        }
    }
}
