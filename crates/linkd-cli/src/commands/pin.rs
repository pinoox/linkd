use std::path::PathBuf;

use crossterm::style::Stylize;
use linkd_adapters::{adapter_for, detect_ecosystem};
use linkd_core::{display_path, normalize_path, Ecosystem};
use linkd_registry::PinnedStore;

pub async fn run(
    path: Option<PathBuf>,
    name: Option<String>,
    ecosystem: Option<Ecosystem>,
) -> anyhow::Result<()> {
    let raw_path = path.unwrap_or_else(|| PathBuf::from("."));
    let abs_path = normalize_path(&raw_path);

    if !abs_path.exists() {
        anyhow::bail!("directory does not exist: {}", display_path(&abs_path));
    }

    let eco = ecosystem.unwrap_or_else(|| detect_ecosystem(&abs_path, &abs_path));
    let pkg_name = if let Some(custom_name) = name {
        custom_name
    } else {
        let adapter = adapter_for(eco);
        adapter.package_name(&abs_path).map_err(|e| {
            anyhow::anyhow!(
                "could not auto-detect package name in {}: {e}",
                display_path(&abs_path)
            )
        })?
    };

    let store = PinnedStore::default();
    let pinned = store.pin(pkg_name.clone(), abs_path.clone(), eco)?;

    println!(
        "{} {}",
        "✓ Registered reusable package:".green().bold(),
        pinned.name.clone().cyan().bold()
    );
    println!("  path     : {}", display_path(&pinned.path));
    println!("  ecosystem: {:?}", pinned.ecosystem);
    println!();
    println!(
        "  {} run {} in any consumer project directory.",
        "Usage:".yellow().bold(),
        format!("linkd use {}", pinned.name).cyan().bold()
    );

    Ok(())
}
