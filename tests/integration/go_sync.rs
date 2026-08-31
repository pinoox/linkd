use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn go_module_vendor_sync_works() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("go-lib");
    let consumer = tmp.path().join("go-app");

    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("go.mod"),
        b"module example.com/acme/go-lib\n\ngo 1.22\n",
    )
    .unwrap();
    fs::write(
        source.join("helper.go"),
        b"package golib\n\nfunc Message() string { return \"go-lib-synced\" }\n",
    )
    .unwrap();

    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("go.mod"),
        b"module example.com/acme/go-app\n\ngo 1.22\n",
    )
    .unwrap();

    let resolved = resolve_link(&source, &consumer, None, None).unwrap();
    assert_eq!(resolved.ecosystem, Ecosystem::Go);
    assert_eq!(resolved.package_name, "example.com/acme/go-lib");
    assert_eq!(
        resolved.sync_target,
        consumer
            .join("vendor")
            .join("example.com")
            .join("acme")
            .join("go-lib")
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
    let helper_go = resolved.sync_target.join("helper.go");
    assert!(helper_go.exists(), "synced helper.go must exist");
    assert_eq!(
        fs::read_to_string(helper_go).unwrap(),
        "package golib\n\nfunc Message() string { return \"go-lib-synced\" }\n"
    );
}
