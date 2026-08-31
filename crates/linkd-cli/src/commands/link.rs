use std::path::PathBuf;

use linkd_adapters_npm::{parse_package_name, PnpmStoreDetector};
use linkd_core::{ensure_home, SyncStrategy};
use linkd_daemon::DaemonService;
use linkd_ipc::IpcClient;
use linkd_registry::{Registry, RegistryStore};

use crate::human::print_result;

pub async fn run(
    source: PathBuf,
    consumer: PathBuf,
    copy: bool,
    hardlink: bool,
    symlink: bool,
) -> anyhow::Result<()> {
    ensure_home().map_err(|e| linkd_core::LinkdError::io(linkd_core::linkd_home(), e))?;

    let strategy = SyncStrategy::from_cli_flags(hardlink, symlink, copy);
    if let Some(warn) = linkd_core::HumanError::strategy_warning(strategy) {
        eprint!("{}", warn.display());
    }

    let source = source.canonicalize().unwrap_or(source);
    let consumer = consumer.canonicalize().unwrap_or(consumer);

    let package_name = print_result(parse_package_name(&source))?;
    let resolved = print_result(PnpmStoreDetector::resolve(&consumer, &package_name))?;

    let entry = Registry::new_link(
        package_name.clone(),
        source,
        consumer.clone(),
        resolved.sync_target.clone(),
        strategy,
        resolved.isolation_mode,
    );

    let store = RegistryStore::default();
    let client = IpcClient::new().ok();

    if let Some(client) = client {
        if client.ping().await.unwrap_or(false) {
            print_result(client.add_link(entry.clone()).await)?;
            print_result(client.trigger_reconcile(Some(entry.id)).await)?;
            println!("✓ Linked {package_name} → {}", consumer.display());
            println!("  sync target: {}", resolved.sync_target.display());
            return Ok(());
        }
    }

    // Daemon not running — persist locally and sync inline
    store.add_link(entry.clone())?;
    let daemon = DaemonService::new(store);
    print_result(daemon.reconciler().reconcile_link(entry.id))?;
    println!("✓ Linked {package_name} → {}", consumer.display());
    println!("  Tip: run `linkd watch` to keep syncing automatically.");
    Ok(())
}
