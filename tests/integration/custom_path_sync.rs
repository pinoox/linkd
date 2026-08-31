//! Custom path linking and nested-path rejection.

use std::fs;

use linkd_adapters::{resolve_link, validate_link_paths};
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn custom_path_sync_works() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("app");
    let source = tmp.path().join("shared");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("data.txt"), b"shared-data").unwrap();

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

    assert!(LinkMarker::read(&target).unwrap().is_some());
    assert_eq!(
        fs::read_to_string(target.join("data.txt")).unwrap(),
        "shared-data"
    );
}

#[test]
fn nested_paths_rejected_at_resolve() {
    let tmp = TempDir::new().unwrap();
    let parent = tmp.path().join("parent");
    let child = parent.join("child");
    fs::create_dir_all(&child).unwrap();

    assert!(validate_link_paths(&parent, &child).is_err());
    assert!(resolve_link(&parent, &parent, Some(Ecosystem::Custom), Some(&child)).is_err());
}
