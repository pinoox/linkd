use crate::ui::wizard_ui::run_wizard_ui;

pub async fn run() -> anyhow::Result<()> {
    let result = run_wizard_ui()?;

    let Some(result) = result else {
        println!("Wizard cancelled.");
        return Ok(());
    };

    super::link::run(
        result.source,
        result.consumer,
        result.target,
        Some(result.ecosystem),
        false,
        false,
        false,
        !result.start_daemon,
    )
    .await
}
