use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn cargo_crate_vendor_sync_works() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("rust-lib");
    let consumer = tmp.path().join("rust-app");

    fs::create_dir_all(source.join("src")).unwrap();
    fs::write(
        source.join("Cargo.toml"),
        br#"[package]
name = "rust-lib"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(
        source.join("src").join("lib.rs"),
        b"pub fn message() -> &'static str { \"rust-lib-synced\" }\n",
    )
    .unwrap();

    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("Cargo.toml"),
        br#"[package]
name = "rust-app"
version = "0.1.0"
edition = "2021"

[dependencies]
rust-lib = "0.1.0"
"#,
    )
    .unwrap();

    let resolved = resolve_link(&source, &consumer, None, None).unwrap();
    assert_eq!(resolved.ecosystem, Ecosystem::Cargo);
    assert_eq!(resolved.package_name, "rust-lib");
    assert_eq!(
        resolved.sync_target,
        consumer.join("vendor").join("rust-lib")
    );

    let store = RegistryStore::new(tmp.path().join("registry.json"));
    let entry = Registry::new_link(NewLinkParams {
        package_name: resolved.package_name,
        source_path: source.clone(),
        consumer_root: consumer.clone(),
        sync_target: resolved.sync_target.clone(),
        ecosystem: resolved.ecosystem,
        link_mode: LinkMode::PackageManager,
        custom_target: None,
        detected_pm: resolved.detected_pm,
        strategy: SyncStrategy::Copy,
        isolation_mode: IsolationMode::ProjectLocal,
    });
    store.add_link(entry.clone()).unwrap();

    let reconciler = Reconciler::new(RegistryStore::new(store.path().to_path_buf()));
    reconciler.reconcile_link(entry.id).unwrap();

    assert!(LinkMarker::read(&resolved.sync_target).unwrap().is_some());
    let lib_rs = resolved.sync_target.join("src").join("lib.rs");
    assert!(lib_rs.exists(), "synced lib.rs must exist");
    assert_eq!(
        fs::read_to_string(lib_rs).unwrap(),
        "pub fn message() -> &'static str { \"rust-lib-synced\" }\n"
    );
}
