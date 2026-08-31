use crossterm::style::Stylize;
use linkd_core::display_path;
use linkd_registry::PinnedStore;

pub async fn run() -> anyhow::Result<()> {
    let store = PinnedStore::default();
    let packages = store.list()?;

    if packages.is_empty() {
        println!("{}", "No globally registered packages.".dark_grey());
        println!(
            "Navigate to any package folder and run `{}` to register it.",
            "linkd register".cyan().bold()
        );
        return Ok(());
    }

    println!("{}", "Globally registered packages:".yellow().bold());
    println!();
    for pkg in packages {
        println!(
            "  📦 {:<24} {:<12} {}",
            pkg.name.cyan().bold(),
            format!("[{:?}]", pkg.ecosystem).green(),
            display_path(&pkg.path).dark_grey()
        );
    }
    println!();
    println!(
        "Use in any project with: {}",
        "linkd use <package-name>".cyan().bold()
    );
    Ok(())
}
