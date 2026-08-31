use inquire::{Confirm, Text};

pub async fn run() -> anyhow::Result<()> {
    println!("linkd init — first-time setup\n");

    let source = Text::new("Source package directory:")
        .with_default("./packages/my-lib")
        .prompt()?;

    let consumer = Text::new("Consumer project directory:")
        .with_default("../my-app")
        .prompt()?;

    let consumer_path = std::path::PathBuf::from(&consumer);
    let pm = linkd_adapters_npm::detect_package_manager(&consumer_path);
    println!("✓ Detected package manager: {pm:?}");

    #[cfg(target_os = "macos")]
    let strategy_hint = "reflink (APFS)";
    #[cfg(not(target_os = "macos"))]
    let strategy_hint = "copy";

    println!("✓ Suggested sync strategy: {strategy_hint}");

    let proceed = Confirm::new("Create link now?")
        .with_default(true)
        .prompt()?;

    if proceed {
        super::link::run(
            source.into(),
            consumer.into(),
            false,
            false,
            false,
        )
        .await?;
        println!("\n✓ Done. Run `linkd watch` to start the daemon with live UI.");
    }

    Ok(())
}
