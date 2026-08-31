//! Simulates composer install overwriting vendor and verifies reconcile restores dev copy.

use std::fs;

use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, LinkSyncStatus, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn composer_reinstall_simulation_restores_marker_and_content() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("app");
    let source = tmp.path().join("php-lib");
    fs::create_dir_all(source.join("src")).unwrap();
    fs::write(
        source.join("composer.json"),
        br#"{"name":"acme/php-lib","autoload":{"psr-4":{"Acme\\":"src/"}}}"#,
    )
    .unwrap();
    fs::write(
        source.join("src").join("Lib.php"),
        b"<?php namespace Acme; class Lib { public function v() { return 'dev'; } }",
    )
    .unwrap();

    let target = consumer.join("vendor").join("acme").join("php-lib");
    let store = RegistryStore::new(tmp.path().join("registry.json"));

    let entry = Registry::new_link(NewLinkParams {
        package_name: "acme/php-lib".into(),
        source_path: source.clone(),
        consumer_root: consumer.clone(),
        sync_target: target.clone(),
        ecosystem: Ecosystem::Composer,
        link_mode: LinkMode::PackageManager,
        custom_target: None,
        detected_pm: Some("composer".into()),
        strategy: SyncStrategy::Copy,
        isolation_mode: IsolationMode::ProjectLocal,
    });
    store.add_link(entry.clone()).unwrap();

    let reconciler = Reconciler::new(RegistryStore::new(store.path().to_path_buf()));
    reconciler.reconcile_link(entry.id).unwrap();

    assert!(LinkMarker::read(&target).unwrap().is_some());
    let content = fs::read_to_string(target.join("src").join("Lib.php")).unwrap();
    assert!(content.contains("dev"));

    let _ = fs::remove_dir_all(&target);
    fs::create_dir_all(target.join("src")).unwrap();
    fs::write(
        target.join("src").join("Lib.php"),
        b"<?php namespace Acme; class Lib { public function v() { return 'registry'; } }",
    )
    .unwrap();
    assert!(LinkMarker::read(&target).unwrap().is_none());

    reconciler.reconcile_link(entry.id).unwrap();

    assert!(LinkMarker::read(&target).unwrap().is_some());
    let restored = fs::read_to_string(target.join("src").join("Lib.php")).unwrap();
    assert!(restored.contains("dev"));

    let reg = store.load().unwrap();
    let link = Registry::find_by_id(&reg.links, entry.id).unwrap();
    assert_eq!(link.last_sync_status, LinkSyncStatus::Synced);
}
