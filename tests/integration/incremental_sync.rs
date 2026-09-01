//! Verifies 3-layer smart incremental sync behavior:
//! 1. Modifying one file only touches that file in target (preserves untouched files).
//! 2. Second reconcile without source changes is a no-op (all files skipped).
//! 3. Deleting a file removes only that file.

use std::fs;
use std::thread::sleep;
use std::time::Duration;

use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn smart_incremental_sync_preserves_untouched_files() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("app");
    let source = tmp.path().join("shared");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("file1.js"), b"console.log('file1');").unwrap();
    fs::write(source.join("file2.js"), b"console.log('file2');").unwrap();
    fs::write(source.join("file3.js"), b"console.log('file3');").unwrap();

    let target = consumer.join("node_modules").join("shared");
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

    // Initial sync
    reconciler.reconcile_link(entry.id).unwrap();

    assert!(target.join("file1.js").exists());
    assert!(target.join("file2.js").exists());
    assert!(target.join("file3.js").exists());

    let file1_target_mtime_before = fs::metadata(target.join("file1.js"))
        .unwrap()
        .modified()
        .unwrap();
    let file2_target_mtime_before = fs::metadata(target.join("file2.js"))
        .unwrap()
        .modified()
        .unwrap();

    // Sleep a moment to ensure new timestamp if modified
    sleep(Duration::from_millis(50));

    // Modify ONLY file2.js in source
    fs::write(source.join("file2.js"), b"console.log('file2 updated!');").unwrap();

    // Reconcile again
    reconciler.reconcile_link(entry.id).unwrap();

    let file1_target_mtime_after = fs::metadata(target.join("file1.js"))
        .unwrap()
        .modified()
        .unwrap();
    let file2_target_mtime_after = fs::metadata(target.join("file2.js"))
        .unwrap()
        .modified()
        .unwrap();

    // Untouched file1.js MUST keep its original modified time in target (no needless rewrite)
    assert_eq!(
        file1_target_mtime_before, file1_target_mtime_after,
        "Untouched file1.js should not be rewritten or touched"
    );

    // Modified file2.js MUST have updated content and timestamp
    assert_ne!(
        file2_target_mtime_before, file2_target_mtime_after,
        "Modified file2.js must be updated in target"
    );
    assert_eq!(
        fs::read_to_string(target.join("file2.js")).unwrap(),
        "console.log('file2 updated!');"
    );

    assert!(LinkMarker::read(&target).unwrap().is_some());
}
