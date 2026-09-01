use std::fs;
use std::sync::Arc;
use std::time::Duration;

use linkd_core::{Ecosystem, IsolationMode, LinkSyncStatus, LinkdResult, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_ipc::{ensure_auth_token, DaemonEvent, IpcClient, IpcServer};
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;
use tokio::sync::{broadcast, Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_ipc_event_streaming_and_controls() -> LinkdResult<()> {
    let _guard = ENV_LOCK.lock().await;
    let tmp = TempDir::new().unwrap();
    std::env::set_var("LINKD_HOME", tmp.path());

    let _ = ensure_auth_token();

    let reg_store = RegistryStore::new(tmp.path().join("registry.json"));

    let source = tmp.path().join("my-lib");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("package.json"),
        br#"{"name": "my-lib", "version": "1.0.0"}"#,
    )
    .unwrap();
    fs::write(source.join("index.js"), b"module.exports = 1;\n").unwrap();

    let consumer = tmp.path().join("my-app");
    let target = consumer.join("node_modules").join("my-lib");
    fs::create_dir_all(&target).unwrap();

    let entry = Registry::new_link(NewLinkParams {
        package_name: "my-lib".into(),
        source_path: source.clone(),
        consumer_root: consumer.clone(),
        sync_target: target.clone(),
        ecosystem: Ecosystem::Npm,
        link_mode: linkd_core::LinkMode::PackageManager,
        custom_target: None,
        detected_pm: None,
        strategy: SyncStrategy::Copy,
        isolation_mode: IsolationMode::ProjectLocal,
    });

    reg_store.add_link(entry.clone())?;

    let (events_tx, _) = broadcast::channel::<DaemonEvent>(128);

    let reconciler = Arc::new(
        Reconciler::new(RegistryStore::new(reg_store.path().to_path_buf()))
            .with_events_tx(events_tx.clone()),
    );

    let reconciler_for_hook = reconciler.clone();
    let hook = Arc::new(move |link_id: Option<uuid::Uuid>| {
        if let Some(id) = link_id {
            let _ = reconciler_for_hook.reconcile_link(id);
        } else {
            let _ = reconciler_for_hook.reconcile_all();
        }
    });

    let ipc_server = IpcServer::new(RegistryStore::new(reg_store.path().to_path_buf()))
        .with_reconcile_hook(hook)
        .with_events_tx(events_tx.clone());

    tokio::spawn(async move {
        let _ = ipc_server.run().await;
    });

    let mut connected = false;
    let client = IpcClient::new()?;
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if client.ping().await.unwrap_or(false) {
            connected = true;
            break;
        }
    }
    assert!(connected, "IPC client failed to connect to test server");

    let mut rx = client.subscribe_events().await?;

    // 1. Initial snapshot
    let initial = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout on initial snapshot")
        .expect("received event");

    match initial {
        DaemonEvent::Snapshot { snapshot } => {
            assert_eq!(snapshot.links.len(), 1);
            assert_eq!(snapshot.links[0].package_name, "my-lib");
        }
        other => panic!("expected Snapshot, got {other:?}"),
    }

    // 2. Trigger reconcile and verify events
    client.trigger_reconcile(Some(entry.id)).await?;

    let mut saw_started = false;
    let mut saw_completed = false;
    let mut saw_synced_status = false;

    for _ in 0..5 {
        if let Ok(Some(evt)) = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
            match evt {
                DaemonEvent::SyncStarted { package_name, .. } if package_name == "my-lib" => {
                    saw_started = true;
                }
                DaemonEvent::SyncCompleted { package_name, .. } if package_name == "my-lib" => {
                    saw_completed = true;
                }
                DaemonEvent::LinkStatusChanged {
                    package_name,
                    status,
                    ..
                } if package_name == "my-lib" && status == LinkSyncStatus::Synced => {
                    saw_synced_status = true;
                }
                _ => {}
            }
            if saw_started && saw_completed && saw_synced_status {
                break;
            }
        }
    }

    assert!(saw_started, "expected SyncStarted event");
    assert!(saw_completed, "expected SyncCompleted event");
    assert!(saw_synced_status, "expected LinkStatusChanged(Synced)");

    // 3. Toggle pause
    client.toggle_pause_link("my-lib").await?;

    let mut saw_paused = false;
    for _ in 0..10 {
        if let Ok(Some(DaemonEvent::LinkStatusChanged {
            package_name,
            status,
            ..
        })) = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await
        {
            if package_name == "my-lib" && status == LinkSyncStatus::Paused {
                saw_paused = true;
                break;
            }
        }
    }
    assert!(saw_paused, "expected LinkStatusChanged(Paused)");

    // Check registry status is Paused
    let status = client.status().await?;
    assert_eq!(status.links[0].last_sync_status, LinkSyncStatus::Paused);

    // 4. Toggle resume
    client.toggle_pause_link("my-lib").await?;

    let mut saw_resumed = false;
    for _ in 0..10 {
        if let Ok(Some(DaemonEvent::LinkStatusChanged {
            package_name,
            status,
            ..
        })) = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await
        {
            if package_name == "my-lib" && status == LinkSyncStatus::Pending {
                saw_resumed = true;
                break;
            }
        }
    }
    assert!(saw_resumed, "expected LinkStatusChanged(Pending)");

    std::env::remove_var("LINKD_HOME");
    Ok(())
}
