//! Concurrent readers should never observe a missing package directory during atomic swap.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use linkd_core::{IsolationMode, SyncStrategy};
use linkd_sync::{SyncEngine, WriteAllowlist};
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn atomic_swap_under_load() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("app");
    let target = consumer.join("node_modules").join("pkg");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("v1.js"), b"v1").unwrap();

    let source = tmp.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("package.json"), br#"{"name":"pkg"}"#).unwrap();
    fs::write(source.join("v2.js"), b"v2").unwrap();

    let allowlist = WriteAllowlist::from_consumer(&consumer, vec![]);
    let engine = SyncEngine::new(allowlist);

    let stop = Arc::new(AtomicBool::new(false));
    let saw_missing = Arc::new(AtomicBool::new(false));
    let target_for_reader = target.clone();

    let stop_reader = stop.clone();
    let saw_missing_reader = saw_missing.clone();
    let reader = thread::spawn(move || {
        while !stop_reader.load(Ordering::Relaxed) {
            if !target_for_reader.exists() {
                saw_missing_reader.store(true, Ordering::Relaxed);
            }
            if target_for_reader.exists() {
                let _ = fs::read_dir(&target_for_reader);
            }
            thread::sleep(Duration::from_millis(1));
        }
    });

    for _ in 0..5 {
        engine
            .sync(
                Uuid::new_v4(),
                &source,
                &target,
                &[PathBuf::from("package.json"), PathBuf::from("v2.js")],
                SyncStrategy::Copy,
                IsolationMode::ProjectLocal,
            )
            .unwrap();
    }

    stop.store(true, Ordering::Relaxed);
    reader.join().unwrap();

    assert!(!saw_missing.load(Ordering::Relaxed), "target path was missing during swap");
    assert!(target.join("v2.js").exists());
}
