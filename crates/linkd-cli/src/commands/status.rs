use linkd_daemon::{is_daemon_running, read_daemon_pid};
use linkd_ipc::IpcClient;
use linkd_registry::RegistryStore;

pub async fn run(json: bool) -> anyhow::Result<()> {
    let pid_info = read_daemon_pid().ok().flatten();

    if let Ok(client) = IpcClient::new() {
        if client.ping().await.unwrap_or(false) {
            let snap = client.status().await?;
            if json {
                let out = serde_json::json!({
                    "daemon_running": true,
                    "pid": pid_info.as_ref().map(|p| p.pid),
                    "links": snap.links,
                    "pm_install_hint": snap.pm_install_hint,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                if let Some(pid) = pid_info {
                    println!("Daemon: running (pid {})", pid.pid);
                } else {
                    println!("Daemon: running");
                }
                if let Some(hint) = snap.pm_install_hint {
                    println!("PM: {hint}");
                }
                for link in snap.links {
                    println!(
                        "- {} → {} [{:?}] ({:?})",
                        link.package_name,
                        link.consumer_root.display(),
                        link.ecosystem,
                        link.last_sync_status
                    );
                }
            }
            return Ok(());
        }
    }

    let reg = RegistryStore::default().load()?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "daemon_running": is_daemon_running(),
                "pid": pid_info.as_ref().map(|p| p.pid),
                "links": reg.links,
            }))?
        );
    } else {
        println!(
            "Daemon: {}",
            if is_daemon_running() {
                "running (IPC unreachable)"
            } else {
                "not running"
            }
        );
        for link in reg.links {
            println!(
                "- {} → {} [{:?}] (registry only)",
                link.package_name,
                link.consumer_root.display(),
                link.last_sync_status
            );
        }
    }
    Ok(())
}
