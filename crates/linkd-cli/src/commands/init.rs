use inquire::{Confirm, Select, Text};
use linkd_core::Ecosystem;

pub async fn run() -> anyhow::Result<()> {
    println!("linkd init — quick setup\n");

    let link_type = Select::new(
        "Link type:",
        vec!["npm package", "composer package", "custom path"],
    )
    .prompt()?;

    let source = Text::new("Source package directory:")
        .with_default("./packages/my-lib")
        .prompt()?;

    let consumer = Text::new("Consumer project directory:")
        .with_default("../my-app")
        .prompt()?;

    let (target, ecosystem) = if link_type == "custom path" {
        let target = Text::new("Sync target path (inside consumer):")
            .with_default("./lib/shared")
            .prompt()?;
        (Some(target.into()), Some(Ecosystem::Custom))
    } else {
        let eco = if link_type == "composer package" {
            Ecosystem::Composer
        } else {
            Ecosystem::Npm
        };
        (None, Some(eco))
    };

    #[cfg(target_os = "macos")]
    let strategy_hint = "reflink (APFS)";
    #[cfg(not(target_os = "macos"))]
    let strategy_hint = "copy";

    println!("✓ Suggested sync strategy: {strategy_hint}");

    let start_daemon = Confirm::new("Start background daemon after linking?")
        .with_default(true)
        .prompt()?;

    let proceed = Confirm::new("Create link now?")
        .with_default(true)
        .prompt()?;

    if proceed {
        super::link::run(
            source.into(),
            consumer.into(),
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
