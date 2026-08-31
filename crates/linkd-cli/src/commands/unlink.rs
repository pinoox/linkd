use linkd_ipc::IpcClient;
use linkd_registry::RegistryStore;

use crate::human::print_result;

pub async fn run(target: &str) -> anyhow::Result<()> {
    if let Ok(client) = IpcClient::new() {
        if client.ping().await.unwrap_or(false) {
            print_result(client.remove_link(target).await.map_err(|e| e))?;
            println!("✓ Unlinked {target}");
            return Ok(());
        }
    }

    let store = RegistryStore::default();
    if store.remove_link(target)?.is_some() {
        println!("✓ Unlinked {target}");
        Ok(())
    } else {
        anyhow::bail!("no link found for {target}")
    }
}
