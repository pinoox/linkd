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
    Dart,
    Flutter,
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
            EcosystemArg::Dart | EcosystemArg::Flutter => Ecosystem::Dart,
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
    /// Register the current package directory globally for easy reuse
    #[command(alias = "pin", alias = "add", alias = "publish")]
    Register {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum)]
        ecosystem: Option<EcosystemArg>,
    },
    /// Link a globally registered package into the current project
    #[command(alias = "on", alias = "attach")]
    Use {
        package_name: String,
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
    /// List all globally registered packages
    #[command(alias = "pinned")]
    Packages,
    /// Unregister a globally registered package
    #[command(alias = "unpin")]
    Unregister { package_name: String },
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
    /// Interactive guided setup wizard
    #[command(alias = "wizard")]
    Init,
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
        Commands::Register {
            path,
            name,
            ecosystem,
        } => commands::pin::run(Some(path), name, ecosystem.map(Into::into)).await,
        Commands::Use {
            package_name,
            consumer,
            target,
            ecosystem,
            copy,
            hardlink,
            link,
            no_daemon,
        } => {
            commands::use_pkg::run(
                package_name,
                Some(consumer),
                target,
                ecosystem.map(Into::into),
                copy,
                hardlink,
                link,
                no_daemon,
            )
            .await
        }
        Commands::Packages => commands::packages::run().await,
        Commands::Unregister { package_name } => commands::unpin::run(&package_name).await,
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
        Commands::Version { json } => {
            if json {
                let info = serde_json::json!({
                    "name": "linkd",
                    "version": env!("CARGO_PKG_VERSION"),
                    "target_os": std::env::consts::OS,
                    "target_arch": std::env::consts::ARCH,
                    "ecosystems": ["npm", "pnpm", "yarn", "bun", "composer", "python", "go", "cargo", "jvm", "dart", "flutter", "custom"]
                });
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("linkd v{}", env!("CARGO_PKG_VERSION"));
                println!("Continuous local-dev link daemon for multi-ecosystem monorepos");
                println!("Supported ecosystems: JS/TS, PHP Composer, Python, Go, Rust Cargo, JVM, Dart/Flutter, Custom");
            }
            Ok(())
        }
        Commands::Completions { shell } => {
            generate(shell, &mut Cli::command(), "linkd", &mut std::io::stdout());
            Ok(())
        }
    }
}
