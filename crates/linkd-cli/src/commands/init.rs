use std::path::PathBuf;

use crossterm::style::Stylize;
use inquire::{Confirm, Select, Text};
use linkd_core::{clean_path, display_path, normalize_path, Ecosystem};

pub async fn run() -> anyhow::Result<()> {
    println!();
    println!(
        "  {} {}",
        "⚡ linkd init".cyan().bold(),
        "— Interactive Setup".dark_grey()
    );
    println!();

    let link_type = Select::new(
        "Select package ecosystem:",
        vec![
            "JavaScript / TypeScript (npm, pnpm, yarn, bun)",
            "Flutter / Dart (pubspec.yaml)",
            "PHP (Composer)",
            "Python (uv, pip, poetry)",
            "Go (go.mod / vendor)",
            "Rust (Cargo / vendor)",
            "Java / Kotlin (JVM)",
            "Custom target directory",
        ],
    )
    .prompt()?;

    let (target, ecosystem) = match link_type {
        "Custom target directory" => {
            let target = Text::new("Sync target path (inside consumer):")
                .with_default("./lib/shared")
                .prompt()?;
            (Some(PathBuf::from(target)), Some(Ecosystem::Custom))
        }
        "Flutter / Dart (pubspec.yaml)" => (None, Some(Ecosystem::Dart)),
        "PHP (Composer)" => (None, Some(Ecosystem::Composer)),
        "Python (uv, pip, poetry)" => (None, Some(Ecosystem::Python)),
        "Go (go.mod / vendor)" => (None, Some(Ecosystem::Go)),
        "Rust (Cargo / vendor)" => (None, Some(Ecosystem::Cargo)),
        "Java / Kotlin (JVM)" => (None, Some(Ecosystem::Jvm)),
        _ => (None, Some(Ecosystem::Npm)),
    };

    let source_raw = Text::new("Source package directory:")
        .with_default("./packages/my-lib")
        .prompt()?;
    let source_path = clean_path(&normalize_path(&PathBuf::from(source_raw)));

    let consumer_raw = Text::new("Consumer project directory:")
        .with_default(".")
        .prompt()?;
    let consumer_path = clean_path(&normalize_path(&PathBuf::from(consumer_raw)));

    println!();
    println!(
        "  {} {}",
        "Source  :".white().bold(),
        display_path(&source_path).cyan()
    );
    println!(
        "  {} {}",
        "Consumer:".white().bold(),
        display_path(&consumer_path).cyan()
    );

    #[cfg(target_os = "macos")]
    let strategy_hint = "reflink (APFS CoW)";
    #[cfg(not(target_os = "macos"))]
    let strategy_hint = "copy (atomic CoW/clone)";

    println!("  {} {}", "Strategy:".white().bold(), strategy_hint.green());
    println!();

    let start_daemon = Confirm::new("Start background daemon after linking?")
        .with_default(true)
        .prompt()?;

    let proceed = Confirm::new("Create and synchronize link now?")
        .with_default(true)
        .prompt()?;

    if proceed {
        super::link::run(
            source_path,
            consumer_path,
            target,
            ecosystem,
            false,
            false,
            false,
            !start_daemon,
        )
        .await?;
    }

    Ok(())
}
