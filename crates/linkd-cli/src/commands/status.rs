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
                        linkd_core::display_path(&link.consumer_root),
                        link.ecosystem,
                        link.last_sync_status
                    );
                }
            }
            return Ok(());
        }
    }

    let reg = RegistryStore::default().load()?;
    let running = is_daemon_running();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "daemon_running": running,
                "ipc_connected": false,
                "pid": pid_info.as_ref().map(|p| p.pid),
                "links": reg.links,
            }))?
        );
    } else if let Some(pid) = pid_info {
        println!("Daemon: not responding (pid {}, IPC unreachable)", pid.pid);
        println!("  Warning: Process exists but is not responding to IPC.");
        println!("  Run `linkd stop` or restart with `linkd start`.");
        for link in reg.links {
            println!(
                "- {} → {} [{:?}] (unresponsive)",
                link.package_name,
                linkd_core::display_path(&link.consumer_root),
                link.ecosystem
            );
        }
    } else {
        println!("Daemon: not running");
        println!("  Live sync is inactive. Run `linkd start` or `linkd use <package>` to start the daemon.");
        for link in reg.links {
            println!(
                "- {} → {} [{:?}] (inactive)",
                link.package_name,
                linkd_core::display_path(&link.consumer_root),
                link.ecosystem
            );
        }
    }
    Ok(())
}
