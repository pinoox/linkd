use std::collections::BTreeMap;

use crossterm::style::Stylize;
use linkd_core::{display_path, Ecosystem, LinkSyncStatus};
use linkd_ipc::IpcClient;
use linkd_registry::RegistryStore;

pub async fn run() -> anyhow::Result<()> {
    let links = if let Ok(client) = IpcClient::new() {
        if client.ping().await.unwrap_or(false) {
            client.list_links().await?
        } else {
            RegistryStore::default().load()?.links
        }
    } else {
        RegistryStore::default().load()?.links
    };

    if links.is_empty() {
        println!();
        println!("  {}", "No active synchronized links.".dark_grey());
        println!("  Use: {}", "linkd link <source> [consumer]".cyan());
        println!("  Or:  {}", "linkd use <package>".cyan());
        println!();
        return Ok(());
    }

    println!();
    println!(
        "  {} {} {}",
        "🔗 Active Synchronized Links".cyan().bold(),
        format!("({})", links.len()).dark_grey(),
        "— hierarchical view".dark_grey()
    );
    println!();

    // Group links by package_name
    let mut grouped: BTreeMap<String, Vec<&linkd_core::LinkEntry>> = BTreeMap::new();
    for link in &links {
        grouped
            .entry(link.package_name.clone())
            .or_default()
            .push(link);
    }

    for (pkg_name, pkg_links) in &grouped {
        let first = pkg_links[0];
        let eco_str = match first.ecosystem {
            Ecosystem::Npm => "[npm]".red(),
            Ecosystem::Composer => "[composer]".magenta(),
            Ecosystem::Python => "[python]".yellow(),
            Ecosystem::Go => "[go]".cyan(),
            Ecosystem::Cargo => "[cargo]".red(),
            Ecosystem::Jvm => "[jvm]".green(),
            Ecosystem::Dart => "[dart/flutter]".cyan(),
            Ecosystem::Dotnet => "[dotnet]".magenta(),
            Ecosystem::Ruby => "[ruby]".red(),
            Ecosystem::Swift => "[swift]".yellow(),
            Ecosystem::Elixir => "[elixir]".magenta(),
            Ecosystem::Custom => "[custom]".blue(),
        };

        let consumers_count = pkg_links.len();
        let consumer_label = if consumers_count == 1 {
            "1 consumer".to_string()
        } else {
            format!("{consumers_count} consumers")
        };

        println!(
            "  {} {} {} {}",
            "📦".cyan(),
            pkg_name.as_str().white().bold(),
            eco_str,
            format!("({consumer_label})").dark_grey()
        );
        println!(
            "     {} {}",
            "Source:".dark_grey(),
            display_path(&first.source_path).dark_grey()
        );

        for (idx, link) in pkg_links.iter().enumerate() {
            let is_last = idx + 1 == pkg_links.len();
            let branch = if is_last { "└──" } else { "├──" };

            let status_styled = match link.last_sync_status {
                LinkSyncStatus::Synced => "✓ synced".green(),
                LinkSyncStatus::Syncing => "⏳ syncing".yellow().bold(),
                LinkSyncStatus::Pending => "… pending".yellow(),
                LinkSyncStatus::Error => "✗ error".red().bold(),
                LinkSyncStatus::Paused => "⏸ paused".dark_grey(),
            };

            let files_str = format!("{} files", link.file_count).dark_grey();
            let consumer_display = display_path(&link.consumer_root);
            let target_display = display_path(&link.sync_target);

            println!(
                "     {} 📂 {} → {}  ({:?}, {}) [{}]",
                branch.cyan(),
                consumer_display.cyan().bold(),
                target_display.dark_grey(),
                link.strategy,
                files_str,
                status_styled
            );
        }
        println!();
    }

    Ok(())
}
