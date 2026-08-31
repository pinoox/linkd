use std::path::{Path, PathBuf};

use linkd_adapters::resolve_link;
use linkd_core::{ensure_home, Ecosystem, SyncStrategy};
use linkd_daemon::{ensure_daemon_running, DaemonService};
use linkd_ipc::IpcClient;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};

use crate::human::print_result;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    source: PathBuf,
    consumer: PathBuf,
    target: Option<PathBuf>,
    ecosystem: Option<Ecosystem>,
    copy: bool,
    hardlink: bool,
    symlink: bool,
    no_daemon: bool,
) -> anyhow::Result<()> {
    ensure_home().map_err(|e| linkd_core::LinkdError::io(linkd_core::linkd_home(), e))?;

    let strategy = SyncStrategy::from_cli_flags(hardlink, symlink, copy);
    if let Some(warn) = linkd_core::HumanError::strategy_warning(strategy) {
        eprint!("{}", warn.display());
    }

    let source = source.canonicalize().unwrap_or(source);
    let consumer = consumer.canonicalize().unwrap_or(consumer);

    let custom_target = target.as_deref();
    let resolved = print_result(resolve_link(&source, &consumer, ecosystem, custom_target))?;

    let entry = Registry::new_link(NewLinkParams {
        package_name: resolved.package_name.clone(),
        source_path: source,
        consumer_root: consumer.clone(),
        sync_target: resolved.sync_target.clone(),
        ecosystem: resolved.ecosystem,
        link_mode: resolved.link_mode,
        custom_target: custom_target.map(|p| p.to_path_buf()),
        detected_pm: resolved.detected_pm.clone(),
        strategy,
        isolation_mode: resolved.resolved.isolation_mode,
    });

    if !no_daemon {
        print_result(ensure_daemon_running())?;
    }

    let store = RegistryStore::default();
    let client = IpcClient::new().ok();

    if let Some(client) = client {
        if client.ping().await.unwrap_or(false) {
            print_result(client.add_link(entry.clone()).await)?;
            print_result(client.trigger_reconcile(Some(entry.id)).await)?;
            print_result_sync_message(
                &resolved.package_name,
                &consumer,
                &resolved.sync_target,
                resolved.detected_pm.as_deref(),
            );
            return Ok(());
        }
    }

    store.add_link(entry.clone())?;
    let daemon = DaemonService::new(store);
    print_result(daemon.reconciler().reconcile_link(entry.id))?;
    print_result_sync_message(
        &resolved.package_name,
        &consumer,
        &resolved.sync_target,
        resolved.detected_pm.as_deref(),
    );
    if no_daemon {
        println!("  Tip: run `linkd start` to keep syncing automatically.");
    }
    Ok(())
}

fn print_result_sync_message(
    package_name: &str,
    consumer: &Path,
    sync_target: &Path,
    detected_pm: Option<&str>,
) {
    println!("✓ Linked {package_name} → {}", consumer.display());
    println!("  sync target: {}", sync_target.display());
    if let Some(pm) = detected_pm {
        println!("  detected: {pm}");
    }
}
