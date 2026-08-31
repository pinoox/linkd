use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn jvm_maven_sync_works() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("java-lib");
    let consumer = tmp.path().join("java-app");

    fs::create_dir_all(
        source
            .join("src")
            .join("main")
            .join("java")
            .join("com")
            .join("acme"),
    )
    .unwrap();
    fs::write(
        source.join("pom.xml"),
        br#"<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.acme</groupId>
    <artifactId>java-lib</artifactId>
    <version>0.1.0</version>
</project>
"#,
    )
    .unwrap();
    fs::write(
        source
            .join("src")
            .join("main")
            .join("java")
            .join("com")
            .join("acme")
            .join("Helper.java"),
        b"package com.acme;\npublic class Helper { public static String getMessage() { return \"java-lib-synced\"; } }\n",
    )
    .unwrap();

    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("pom.xml"),
        br#"<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.acme</groupId>
    <artifactId>java-app</artifactId>
    <version>0.1.0</version>
</project>
"#,
    )
    .unwrap();

    let resolved = resolve_link(&source, &consumer, None, None).unwrap();
    assert_eq!(resolved.ecosystem, Ecosystem::Jvm);
    assert_eq!(resolved.package_name, "com.acme:java-lib");
    assert_eq!(resolved.sync_target, consumer.join("libs").join("java-lib"));

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
    let helper_java = resolved
        .sync_target
        .join("src")
        .join("main")
        .join("java")
        .join("com")
        .join("acme")
        .join("Helper.java");
    assert!(helper_java.exists(), "synced Helper.java must exist");
}
