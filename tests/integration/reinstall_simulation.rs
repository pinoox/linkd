//! Simulates npm install overwriting node_modules and verifies reconcile restores marker.

use std::fs;

use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, LinkSyncStatus, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn reinstall_simulation_restores_marker_and_content() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("app");
    let source = tmp.path().join("my-lib");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("package.json"),
        br#"{"name":"my-lib","version":"1.0.0"}"#,
    )
    .unwrap();
    fs::write(source.join("index.js"), b"dev-version").unwrap();

    let target = consumer.join("node_modules").join("my-lib");
    let registry_path = tmp.path().join("registry.json");
    let store = RegistryStore::new(registry_path);

    let entry = Registry::new_link(NewLinkParams {
        package_name: "my-lib".into(),
        source_path: source.clone(),
        consumer_root: consumer.clone(),
        sync_target: target.clone(),
        ecosystem: Ecosystem::Npm,
        link_mode: LinkMode::PackageManager,
        custom_target: None,
        detected_pm: None,
        strategy: SyncStrategy::Copy,
        isolation_mode: IsolationMode::ProjectLocal,
    });
    store.add_link(entry.clone()).unwrap();

    let reconciler = Reconciler::new(RegistryStore::new(store.path().to_path_buf()));
    reconciler.reconcile_link(entry.id).unwrap();

    assert!(LinkMarker::read(&target).unwrap().is_some());
    assert_eq!(
        fs::read_to_string(target.join("index.js")).unwrap(),
        "dev-version"
    );

    // Simulate npm install replacing the package (marker gone, registry version)
    let _ = fs::remove_dir_all(&target);
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("index.js"), b"registry-version").unwrap();
    assert!(LinkMarker::read(&target).unwrap().is_none());

    reconciler.reconcile_link(entry.id).unwrap();

    assert!(LinkMarker::read(&target).unwrap().is_some());
    assert_eq!(
        fs::read_to_string(target.join("index.js")).unwrap(),
        "dev-version"
    );

    let reg = store.load().unwrap();
    let link = Registry::find_by_id(&reg.links, entry.id).unwrap();
    assert_eq!(link.last_sync_status, LinkSyncStatus::Synced);
    assert!(link.last_sync_hash.is_some());
}
