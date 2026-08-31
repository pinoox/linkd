use linkd_daemon::stop_daemon;

pub async fn run(force: bool) -> anyhow::Result<()> {
    stop_daemon(force).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    println!("✓ linkd daemon stopped");
    Ok(())
}
