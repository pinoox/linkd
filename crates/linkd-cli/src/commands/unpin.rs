use crossterm::style::Stylize;
use linkd_registry::PinnedStore;

pub async fn run(package_name: &str) -> anyhow::Result<()> {
    let store = PinnedStore::default();
    if let Some(removed) = store.unpin(package_name)? {
        println!(
            "{} {}",
            "✓ Unregistered global package:".green(),
            removed.name.cyan().bold()
        );
        Ok(())
    } else {
        anyhow::bail!(
            "Package '{}' was not found in registered packages.",
            package_name
        )
    }
}
