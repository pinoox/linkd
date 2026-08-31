use linkd_core::LinkSyncStatus;
use linkd_ipc::IpcClient;
use linkd_registry::RegistryStore;

pub async fn run() -> anyhow::Result<()> {
    let links = if let Ok(client) = IpcClient::new() {
        if client.ping().await.unwrap_or(false) {
            client.list_links().await?
        } else {
            RegistryStore::default().load()?.links
        }
    } else {
        RegistryStore::default().load()?.links
    };

    if links.is_empty() {
        println!("No active links. Use: linkd link <source> [consumer]");
        return Ok(());
    }

    println!("Active links:\n");
    for link in links {
        let status = match link.last_sync_status {
            LinkSyncStatus::Synced => "✓ synced",
            LinkSyncStatus::Syncing => "⏳ syncing",
            LinkSyncStatus::Pending => "… pending",
            LinkSyncStatus::Error => "✗ error",
            LinkSyncStatus::Paused => "⏸ paused",
        };
        println!(
            "  🔗 {} → {} ({:?}) {}",
            link.package_name,
            linkd_core::display_path(&link.consumer_root),
            link.strategy,
            status
        );
    }
    Ok(())
}
