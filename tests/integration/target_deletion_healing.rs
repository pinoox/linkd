use std::fs;
use std::time::Duration;

use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkdResult, SyncStrategy};
use linkd_daemon::DaemonService;
use linkd_ipc::{ensure_auth_token, IpcClient};
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;
use tokio::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_target_deletion_recovers_automatically() -> LinkdResult<()> {
    let _guard = ENV_LOCK.lock().await;
    let tmp = TempDir::new().unwrap();
    std::env::set_var("LINKD_HOME", tmp.path());

    let _ = ensure_auth_token();

    let reg_store = RegistryStore::new(tmp.path().join("registry.json"));

    // 1. Prepare source package
    let source = tmp.path().join("my-lib");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("package.json"),
        br#"{"name": "my-lib", "version": "1.0.0"}"#,
    )
    .unwrap();
    fs::write(source.join("index.js"), b"module.exports = 'active-lib';\n").unwrap();

    // 2. Prepare consumer project
    let consumer = tmp.path().join("my-app");
    let target = consumer.join("node_modules").join("my-lib");
    fs::create_dir_all(consumer.join("node_modules")).unwrap();
    fs::write(
        consumer.join("package.json"),
        br#"{"name": "my-app", "dependencies": {"my-lib": "1.0.0"}}"#,
    )
    .unwrap();

    // 3. Start DaemonService in background
    let service = DaemonService::new(reg_store.clone());
    let daemon_handle = tokio::spawn(async move {
        let _ = service.run_background().await;
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = IpcClient::new()?;
    assert!(client.ping().await?, "daemon should respond to ping");

    // 4. Register link
    let entry = Registry::new_link(NewLinkParams {
        package_name: "my-lib".into(),
        source_path: source.clone(),
        consumer_root: consumer.clone(),
        sync_target: target.clone(),
        ecosystem: Ecosystem::Npm,
        link_mode: linkd_core::LinkMode::PackageManager,
        custom_target: None,
        detected_pm: Some("npm".into()),
        strategy: SyncStrategy::Copy,
        isolation_mode: IsolationMode::ProjectLocal,
    });

    client.add_link(entry).await?;
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Verify initial sync succeeded
    assert!(target.join("index.js").exists());
    assert!(LinkMarker::read(&target).unwrap().is_some());
    assert_eq!(
        fs::read_to_string(target.join("index.js")).unwrap(),
        "module.exports = 'active-lib';\n"
    );

    // 5. Simulating user's exact edge case: rm -rf node_modules/my-lib
    fs::remove_dir_all(&target).unwrap();
    assert!(!target.exists(), "target directory must be deleted");

    // 6. Wait for self-healing (Fast gate within 300ms debounce or 2s heartbeat)
    let mut restored = false;
    for _ in 0..25 {
        tokio::time::sleep(Duration::from_millis(200)).await;
        if target.join("index.js").exists() && LinkMarker::read(&target).unwrap().is_some() {
            if let Ok(content) = fs::read_to_string(target.join("index.js")) {
                if content == "module.exports = 'active-lib';\n" {
                    restored = true;
                    break;
                }
            }
        }
    }
    assert!(
        restored,
        "Daemon failed to self-heal and restore deleted target directory"
    );

    // 7. Cleanup
    let _ = client.shutdown().await;
    let _ = daemon_handle.await;

    std::env::remove_var("LINKD_HOME");
    Ok(())
}
