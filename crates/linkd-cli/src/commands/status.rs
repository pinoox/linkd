use linkd_ipc::IpcClient;
use linkd_registry::RegistryStore;

pub async fn run(json: bool) -> anyhow::Result<()> {
    if let Ok(client) = IpcClient::new() {
        if client.ping().await.unwrap_or(false) {
            let snap = client.status().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&snap)?);
            } else {
                println!("Daemon: running");
                if let Some(hint) = snap.pm_install_hint {
                    println!("PM: {hint}");
                }
                for link in snap.links {
                    println!(
                        "- {} → {} [{:?}]",
                        link.package_name,
                        link.consumer_root.display(),
                        link.last_sync_status
                    );
                }
            }
            return Ok(());
        }
    }

    let reg = RegistryStore::default().load()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&reg.links)?);
    } else {
        println!("Daemon: not running");
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
