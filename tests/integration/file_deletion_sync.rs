//! Verifies deleted source files are removed from sync target after reconcile.

use std::fs;

use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn file_deletion_sync_removes_stale_target_files() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("app");
    let source = tmp.path().join("shared");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("keep.txt"), b"keep").unwrap();
    fs::write(source.join("remove.txt"), b"remove").unwrap();

    let target = consumer.join("lib").join("shared");
    let store = RegistryStore::new(tmp.path().join("registry.json"));

    let entry = Registry::new_link(NewLinkParams {
        package_name: "shared".into(),
        source_path: source.clone(),
        consumer_root: consumer.clone(),
        sync_target: target.clone(),
        ecosystem: Ecosystem::Custom,
        link_mode: LinkMode::CustomPath,
        custom_target: Some(target.clone()),
        detected_pm: None,
        strategy: SyncStrategy::Copy,
        isolation_mode: IsolationMode::ProjectLocal,
    });
    store.add_link(entry.clone()).unwrap();

    let reconciler = Reconciler::new(RegistryStore::new(store.path().to_path_buf()));
    reconciler.reconcile_link(entry.id).unwrap();

    assert!(target.join("keep.txt").exists());
    assert!(target.join("remove.txt").exists());

    fs::remove_file(source.join("remove.txt")).unwrap();
    reconciler.reconcile_link(entry.id).unwrap();

    assert!(target.join("keep.txt").exists());
    assert!(!target.join("remove.txt").exists());
    assert!(LinkMarker::read(&target).unwrap().is_some());

    // Test recursive directory deletion
    let nested_src = source.join("features").join("auth");
    fs::create_dir_all(&nested_src).unwrap();
    fs::write(nested_src.join("login.js"), b"export const login = true;").unwrap();

    reconciler.reconcile_link(entry.id).unwrap();
    assert!(target.join("features/auth/login.js").exists());
    assert!(target.join("features/auth").is_dir());

    // Delete entire features directory from source
    fs::remove_dir_all(source.join("features")).unwrap();
    reconciler.reconcile_link(entry.id).unwrap();

    assert!(!target.join("features/auth/login.js").exists());
    assert!(!target.join("features/auth").exists());
    assert!(!target.join("features").exists());
    assert!(target.join("keep.txt").exists());
}
