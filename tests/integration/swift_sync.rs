use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn swift_package_sync_works() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("SwiftKit");
    let consumer = tmp.path().join("SwiftApp");

    fs::create_dir_all(source.join("Sources").join("SwiftKit")).unwrap();
    fs::write(
        source.join("Package.swift"),
        br#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SwiftKit",
    products: []
)
"#,
    )
    .unwrap();
    fs::write(
        source.join("Sources").join("SwiftKit").join("Kit.swift"),
        b"public struct SwiftKit { public static let name = \"SwiftKit\" }\n",
    )
    .unwrap();

    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("Package.swift"),
        br#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SwiftApp",
    dependencies: []
)
"#,
    )
    .unwrap();

    let resolved = resolve_link(&source, &consumer, None, None).unwrap();
    assert_eq!(resolved.ecosystem, Ecosystem::Swift);
    assert_eq!(resolved.package_name, "SwiftKit");
    assert_eq!(
        resolved.sync_target,
        consumer.join(".build").join("checkouts").join("SwiftKit")
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
    let swift_file = resolved
        .sync_target
        .join("Sources")
        .join("SwiftKit")
        .join("Kit.swift");
    assert!(swift_file.exists(), "synced Kit.swift must exist");
    assert_eq!(
        fs::read_to_string(swift_file).unwrap(),
        "public struct SwiftKit { public static let name = \"SwiftKit\" }\n"
    );
}
