mod commands;
mod human;
mod logging;
mod ui;
mod welcome;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use linkd_core::Ecosystem;
use welcome::print_welcome_guide;

#[derive(Parser)]
#[command(
    name = "linkd",
    version = env!("CARGO_PKG_VERSION"),
    about = "Continuous local-dev link daemon for multi-ecosystem monorepos",
    disable_version_flag = true
)]
struct Cli {
    /// Print version information (-v, -V, --version)
    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version)]
    version: (),

    #[arg(long, hide = true, global = true)]
    daemon_internal: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Clone, Copy, ValueEnum)]
enum EcosystemArg {
    Npm,
    Composer,
    Python,
    Go,
    Cargo,
    Jvm,
    Custom,
}

impl From<EcosystemArg> for Ecosystem {
    fn from(value: EcosystemArg) -> Self {
        match value {
            EcosystemArg::Npm => Ecosystem::Npm,
            EcosystemArg::Composer => Ecosystem::Composer,
            EcosystemArg::Python => Ecosystem::Python,
            EcosystemArg::Go => Ecosystem::Go,
            EcosystemArg::Cargo => Ecosystem::Cargo,
            EcosystemArg::Jvm => Ecosystem::Jvm,
            EcosystemArg::Custom => Ecosystem::Custom,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Link a local package source into a consumer project
    Link {
        source: std::path::PathBuf,
        #[arg(default_value = ".")]
        consumer: std::path::PathBuf,
        #[arg(long)]
        target: Option<std::path::PathBuf>,
        #[arg(long, value_enum)]
        ecosystem: Option<EcosystemArg>,
        #[arg(long)]
        copy: bool,
        #[arg(long)]
        hardlink: bool,
        #[arg(long)]
        link: bool,
        #[arg(long)]
        no_daemon: bool,
    },
    /// Remove an active link
    Unlink { target: String },
    /// List active links
    List,
    /// Start background daemon
    Start,
    /// Stop background daemon
    Stop {
        #[arg(long)]
        force: bool,
    },
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
    /// Live interactive TUI monitor dashboard
    #[command(alias = "top", alias = "dashboard")]
    Monitor {
        #[arg(long)]
        start: bool,
    },
    /// Interactive quick setup wizard
    Init,
    /// Full-screen setup wizard
    Wizard,
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Print version information
    Version {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init()?;
    let cli = Cli::parse();

    if cli.daemon_internal {
        return linkd_daemon::run_daemon_internal()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()));
    }

    let Some(command) = cli.command else {
        print_welcome_guide();
        return Ok(());
    };

    match command {
        Commands::Link {
            source,
            consumer,
            target,
            ecosystem,
            copy,
            hardlink,
            link,
            no_daemon,
        } => {
            commands::link::run(
                source,
                consumer,
                target,
                ecosystem.map(Into::into),
                copy,
                hardlink,
                link,
                no_daemon,
            )
            .await
        }
        Commands::Unlink { target } => commands::unlink::run(&target).await,
        Commands::List => commands::list::run().await,
        Commands::Start => commands::start::run().await,
        Commands::Stop { force } => commands::stop::run(force).await,
        Commands::Watch => commands::watch::run().await,
        Commands::Status { json } => commands::status::run(json).await,
        Commands::Doctor { explain } => commands::doctor::run(explain.as_deref()).await,
        Commands::Logs { follow } => commands::logs::run(follow).await,
        Commands::Monitor { start } => commands::monitor::run(start).await,
        Commands::Init => commands::init::run().await,
        Commands::Wizard => commands::wizard::run().await,
        Commands::Version { json } => {
            if json {
                let info = serde_json::json!({
                    "name": "linkd",
                    "version": env!("CARGO_PKG_VERSION"),
                    "target_os": std::env::consts::OS,
                    "target_arch": std::env::consts::ARCH,
                    "ecosystems": ["npm", "pnpm", "yarn", "bun", "composer", "python", "go", "cargo", "jvm", "custom"]
                });
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("linkd v{}", env!("CARGO_PKG_VERSION"));
                println!("Continuous local-dev link daemon for multi-ecosystem monorepos");
                println!("Supported ecosystems: JS/TS, PHP Composer, Python, Go, Rust Cargo, JVM, Custom");
            }
            Ok(())
        }
        Commands::Completions { shell } => {
            generate(shell, &mut Cli::command(), "linkd", &mut std::io::stdout());
            Ok(())
        }
    }
}
