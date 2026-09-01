use std::path::PathBuf;

use crossterm::style::Stylize;
use linkd_core::{display_path, Ecosystem};
use linkd_registry::PinnedStore;

use super::link;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    package_name: String,
    consumer: Option<PathBuf>,
    target: Option<PathBuf>,
    ecosystem: Option<Ecosystem>,
    copy: bool,
    hardlink: bool,
    symlink: bool,
    no_daemon: bool,
) -> anyhow::Result<()> {
    let store = PinnedStore::default();
    let pinned = match store.get(&package_name)? {
        Some(p) => p,
        None => {
            let all = store.list()?;
            if all.is_empty() {
                anyhow::bail!(
                    "Package '{}' is not registered globally.\nRegister it first by navigating to its directory and running `{}`",
                    package_name.cyan(),
                    "linkd register".cyan().bold()
                );
            } else {
                let list_str = all
                    .iter()
                    .map(|p| {
                        format!(
                            "  • {} ({})",
                            p.name.clone().cyan(),
                            display_path(&p.path).dark_grey()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                anyhow::bail!(
                    "Package '{}' is not registered.\n\nAvailable registered packages:\n{}\n\nTo register this package, navigate to its folder and run `{}`",
                    package_name.cyan(),
                    list_str,
                    "linkd register".cyan().bold()
                );
            }
        }
    };

    let consumer_root = consumer.unwrap_or_else(|| PathBuf::from("."));
    let eco = ecosystem.or(Some(pinned.ecosystem));

    link::run(
        pinned.path,
        consumer_root,
        target,
        eco,
        copy,
        hardlink,
        symlink,
        no_daemon,
    )
    .await
}
