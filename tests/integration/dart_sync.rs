use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn dart_flutter_package_sync_works() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("flutter_lib");
    let consumer = tmp.path().join("flutter_app");

    fs::create_dir_all(source.join("lib")).unwrap();
    fs::write(
        source.join("pubspec.yaml"),
        br#"name: flutter_lib
description: A new Flutter package.
version: 1.0.0
environment:
  sdk: '>=3.0.0 <4.0.0'
"#,
    )
    .unwrap();
    fs::write(
        source.join("lib").join("flutter_lib.dart"),
        b"String getGreeting() => 'flutter_lib_synced';\n",
    )
    .unwrap();

    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("pubspec.yaml"),
        br#"name: flutter_app
description: An awesome Flutter app.
version: 1.0.0
dependencies:
  flutter_lib: ^1.0.0
"#,
    )
    .unwrap();

    let resolved = resolve_link(&source, &consumer, None, None).unwrap();
    assert_eq!(resolved.ecosystem, Ecosystem::Dart);
    assert_eq!(resolved.package_name, "flutter_lib");
    assert_eq!(
        resolved.sync_target,
        consumer
            .join(".dart_tool")
            .join("packages")
            .join("flutter_lib")
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
    let dart_file = resolved.sync_target.join("lib").join("flutter_lib.dart");
    assert!(dart_file.exists(), "synced flutter_lib.dart must exist");
    assert_eq!(
        fs::read_to_string(dart_file).unwrap(),
        "String getGreeting() => 'flutter_lib_synced';\n"
    );
}
