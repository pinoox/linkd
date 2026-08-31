use linkd_daemon::{is_daemon_running, read_daemon_pid, start_daemon};

use crate::ui::monitor_ui::run_monitor_ui;

pub async fn run(start_if_needed: bool) -> anyhow::Result<()> {
    if !is_daemon_running() {
        if start_if_needed {
            println!("Starting linkd background daemon...");
            start_daemon().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        } else {
            anyhow::bail!(
                "linkd daemon is not running.\nStart it with `linkd start` or run `linkd monitor --start`"
            );
        }
    }

    let pid = read_daemon_pid().ok().flatten().map(|p| p.pid);
    run_monitor_ui(pid)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(())
}
