//! Critical: ensure linkd never writes into a simulated pnpm global store.

use std::fs;
use std::path::PathBuf;

use linkd_adapters_npm::PnpmStoreDetector;
use linkd_core::{IsolationMode, LinkdError, SyncStrategy};
use linkd_sync::{SyncEngine, WriteAllowlist};
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn never_writes_directly_to_global_store() {
    let tmp = TempDir::new().unwrap();
    let global_store = tmp.path().join("pnpm-store");
    fs::create_dir_all(&global_store).unwrap();

    let consumer = tmp.path().join("app");
    let nm = consumer.join("node_modules").join("pkg");
    fs::create_dir_all(nm.parent().unwrap()).unwrap();

    let pkg_in_store = global_store.join("v3/files/00/ab/pkg");
    fs::create_dir_all(&pkg_in_store).unwrap();
    fs::write(pkg_in_store.join("index.js"), b"registry").unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(&pkg_in_store, &nm).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&pkg_in_store, &nm).unwrap();

    assert!(PnpmStoreDetector::is_global_store_path_with(
        &pkg_in_store,
        &[global_store.clone()],
    ));

    let allowlist = WriteAllowlist::from_consumer(&consumer, vec![global_store.clone()]);
    let guard_result = allowlist.assert_writable(&pkg_in_store.join("index.js"));
    assert!(matches!(
        guard_result,
        Err(LinkdError::PnpmGlobalStoreForbidden(_))
    ));

    let source = tmp.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("package.json"), br#"{"name":"pkg","version":"1.0.0"}"#).unwrap();
    fs::write(source.join("index.js"), b"dev").unwrap();

    let shadow = consumer.join("node_modules").join(".linkd-shadow").join("pkg");
    let engine = SyncEngine::new(allowlist);

    let out = engine
        .sync(
            Uuid::new_v4(),
            &source,
            &shadow,
            &[PathBuf::from("package.json"), PathBuf::from("index.js")],
            SyncStrategy::Copy,
            IsolationMode::Shadow,
        )
        .expect("sync to shadow");

    assert!(shadow.join("index.js").exists());
    assert_eq!(fs::read_to_string(shadow.join("index.js")).unwrap(), "dev");

    // Global store content must remain unchanged
    assert_eq!(
        fs::read_to_string(pkg_in_store.join("index.js")).unwrap(),
        "registry"
    );

    assert_ne!(out.sync_target, pkg_in_store);
}

#[test]
fn sync_engine_rejects_global_store_target() {
    let tmp = TempDir::new().unwrap();
    let store = tmp.path().join("store");
    fs::create_dir_all(&store).unwrap();
    let target = store.join("pkg");
    fs::create_dir_all(&target).unwrap();

    let consumer = tmp.path().join("app");
    fs::create_dir_all(&consumer.join("node_modules")).unwrap();

    let allowlist = WriteAllowlist::from_consumer(&consumer, vec![store.clone()]);
    let engine = SyncEngine::new(allowlist);

    let source = tmp.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("index.js"), b"x").unwrap();

    let err = engine
        .sync(
            Uuid::new_v4(),
            &source,
            &target,
            &[PathBuf::from("index.js")],
            SyncStrategy::Copy,
            IsolationMode::ProjectLocal,
        )
        .expect_err("sync into global store must fail");

    assert!(matches!(err, LinkdError::PnpmGlobalStoreForbidden(_)));
}
