use std::fs;
use std::time::Duration;

use linkd_core::{Ecosystem, IsolationMode, LinkdResult, SyncStrategy};
use linkd_daemon::DaemonService;
use linkd_ipc::{ensure_auth_token, IpcClient};
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;
use tokio::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_live_idle_watch_sync_propagates_changes() -> LinkdResult<()> {
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
    fs::write(source.join("index.js"), b"module.exports = 'initial';\n").unwrap();

    // 2. Prepare consumer project
    let consumer = tmp.path().join("my-app");
    let target = consumer.join("node_modules").join("my-lib");
    fs::create_dir_all(consumer.join("node_modules")).unwrap();
    fs::write(
        consumer.join("package.json"),
        br#"{"name": "my-app", "dependencies": {"my-lib": "1.0.0"}}"#,
    )
    .unwrap();

    // 3. Start DaemonService in background (started with 0 links initially)
    let service = DaemonService::new(reg_store.clone());
    let daemon_handle = tokio::spawn(async move {
        let _ = service.run_background().await;
    });

    // Allow daemon to initialize and start IPC listener
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = IpcClient::new()?;
    assert!(client.ping().await?, "daemon should respond to ping");

    // 4. Register link while daemon is already running
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
    let initial_content = fs::read_to_string(target.join("index.js")).unwrap();
    assert_eq!(initial_content, "module.exports = 'initial';\n");

    // 5. Idle edit 1: Modify source index.js without running any linkd command
    fs::write(
        source.join("index.js"),
        b"module.exports = 'idle-edit-1';\n",
    )
    .unwrap();

    // Wait for debounced watcher sync (300ms debounce + 500ms tick)
    let mut updated = false;
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if let Ok(content) = fs::read_to_string(target.join("index.js")) {
            if content == "module.exports = 'idle-edit-1';\n" {
                updated = true;
                break;
            }
        }
    }
    assert!(
        updated,
        "Live sync failed to reflect edit 1 in target during idle watch"
    );

    // 6. Idle edit 2: Create a new file in source without running any linkd command
    fs::write(source.join("utils.js"), b"export const helper = 42;\n").unwrap();

    let mut helper_synced = false;
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if let Ok(content) = fs::read_to_string(target.join("utils.js")) {
            if content == "export const helper = 42;\n" {
                helper_synced = true;
                break;
            }
        }
    }
    assert!(
        helper_synced,
        "Live sync failed to reflect new file in target during idle watch"
    );

    // 7. Shutdown daemon cleanly
    let _ = client.shutdown().await;
    let _ = daemon_handle.await;

    std::env::remove_var("LINKD_HOME");
    Ok(())
}
