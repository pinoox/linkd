use linkd_daemon::{is_daemon_running, read_daemon_pid, start_daemon};

pub async fn run() -> anyhow::Result<()> {
    if is_daemon_running() {
        if let Ok(Some(pid)) = read_daemon_pid() {
            println!("✓ linkd daemon already running (pid {})", pid.pid);
        }
        return Ok(());
    }

    start_daemon().map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if let Ok(Some(pid)) = read_daemon_pid() {
        println!("✓ linkd daemon started (pid {})", pid.pid);
        println!("  logs: linkd logs -f");
    } else {
        println!("✓ linkd daemon started");
    }
    Ok(())
}
