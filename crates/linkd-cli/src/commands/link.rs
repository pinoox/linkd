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

    let (source, auto_pkg_name) = if !source.exists() {
        let source_str = source.to_string_lossy();
        if let Ok(Some(pinned)) = linkd_registry::PinnedStore::default().get(&source_str) {
            (pinned.path, Some(pinned.name))
        } else {
            (linkd_core::normalize_path(&source), None)
        }
    } else {
        (linkd_core::normalize_path(&source), None)
    };

    let consumer = linkd_core::normalize_path(&consumer);

    let custom_target = target.as_deref();
    let resolved = print_result(resolve_link(&source, &consumer, ecosystem, custom_target))?;
    let package_name = auto_pkg_name.unwrap_or(resolved.package_name);

    let entry = Registry::new_link(NewLinkParams {
        package_name: package_name.clone(),
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
                &package_name,
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
        &package_name,
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
    println!(
        "✓ Linked {package_name} → {}",
        linkd_core::display_path(consumer)
    );
    println!(
        "  sync target: {}",
        linkd_core::display_path(sync_target)
    );
    if let Some(pm) = detected_pm {
        println!("  detected: {pm}");
    }
}
