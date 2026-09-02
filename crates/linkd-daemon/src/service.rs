use std::sync::{Arc, Mutex};
use std::time::Duration;

use linkd_adapters::completion_markers_for_link;
use linkd_ipc::{IpcServer, ReconcileHook, ShutdownHook};
use linkd_registry::RegistryStore;
use linkd_watcher::{DebouncePool, LinkWatcher, WatchEventKind};
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use tokio::time;
use tracing::{error, info};

use crate::pm_hint::pm_install_hint;
use crate::reconciler::{drain_reconcile_queue, enqueue_reconcile, ReconcileQueue, Reconciler};

pub struct DaemonService {
    registry: RegistryStore,
    reconciler: Reconciler,
    reconcile_queue: ReconcileQueue,
}

impl DaemonService {
    pub fn new(registry: RegistryStore) -> Self {
        let reconciler = Reconciler::new(RegistryStore::new(registry.path().to_path_buf()));
        Self {
            registry,
            reconciler,
            reconcile_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn run_foreground(self) -> linkd_core::LinkdResult<()> {
        self.run_inner(false).await
    }

    pub async fn run_background(self) -> linkd_core::LinkdResult<()> {
        self.run_inner(true).await
    }

    async fn run_inner(self, _background: bool) -> linkd_core::LinkdResult<()> {
        linkd_core::ensure_home()
            .map_err(|e| linkd_core::LinkdError::io(linkd_core::linkd_home(), e))?;

        let queue = self.reconcile_queue.clone();
        let registry_path = self.registry.path().to_path_buf();

        let hook: ReconcileHook = Arc::new(move |link_id| {
            enqueue_reconcile(&queue, link_id);
        });

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let (events_tx, _) = tokio::sync::broadcast::channel::<linkd_ipc::DaemonEvent>(1024);

        let pm_hint = Arc::new(AsyncMutex::new(None));
        let pm_hint_ipc = pm_hint.clone();

        let shutdown_hook: ShutdownHook = {
            let tx = shutdown_tx.clone();
            Arc::new(move || {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let _ = tx.send(()).await;
                });
            })
        };

        let ipc_registry = RegistryStore::new(registry_path.clone());
        let ipc = IpcServer::new(ipc_registry)
            .with_reconcile_hook(hook)
            .with_shutdown_hook(shutdown_hook)
            .with_pm_hint(pm_hint_ipc)
            .with_events_tx(events_tx.clone());

        let reconciler = Reconciler::new(RegistryStore::new(registry_path.clone()))
            .with_events_tx(events_tx.clone());
        let queue_clone = self.reconcile_queue.clone();
        let registry_clone = RegistryStore::new(registry_path.clone());

        let _ = reconciler.reconcile_all();

        let registry_for_watch = self.registry.load().unwrap_or_default();
        let watch_paths = watch_paths_for_links(&registry_for_watch.links);

        let (mut watcher, watch_rx) = LinkWatcher::new(watch_paths)
            .map_err(|e| linkd_core::LinkdError::Other(format!("watcher failed: {e}")))?;

        tokio::spawn(async move {
            if let Err(e) = ipc.run().await {
                error!("ipc server stopped: {e}");
            }
        });

        let mut debounce = DebouncePool::new(300);
        let mut interval = time::interval(Duration::from_millis(500));
        let mut sync_check_counter = 0u32;

        info!("linkd daemon running");

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("linkd daemon shutting down");
                    break;
                }
                _ = interval.tick() => {
                    sync_check_counter += 1;

                    // Drain events from watcher channel into debounce pool
                    while let Ok(event) = watch_rx.try_recv() {
                        let key: String = match event.kind {
                            WatchEventKind::MarkerChanged => "marker".into(),
                            WatchEventKind::TargetChanged => "target".into(),
                            WatchEventKind::SourceChanged => "source".into(),
                        };
                        debounce.push(key, event.path);
                    }

                    for evt in debounce.ready() {
                        // React to source changes, lockfile reinstall markers, and target deletions.
                        // Target writes by the daemon itself are skipped in matching_link_ids_for_event
                        // because sync_target and its marker exist after sync.
                        let should_reconcile =
                            matches!(evt.key.as_str(), "source" | "marker" | "target");
                        if !should_reconcile {
                            continue;
                        }

                        let current_registry = registry_clone.load().unwrap_or_default();
                        let affected_ids =
                            matching_link_ids_for_event(&evt.paths, &current_registry.links);

                        if affected_ids.is_empty() {
                            continue;
                        }

                        info!(
                            "debounced event {:?} paths={} affected_links={}",
                            evt.key,
                            evt.paths.len(),
                            affected_ids.len()
                        );
                        let _ = events_tx.send(linkd_ipc::DaemonEvent::LogMessage {
                            timestamp: chrono::Utc::now(),
                            level: "WATCH".into(),
                            ecosystem: None,
                            message: format!(
                                "event {:?} ({} paths, {} links affected)",
                                evt.key,
                                evt.paths.len(),
                                affected_ids.len()
                            ),
                        });

                        for link_id in affected_ids {
                            enqueue_reconcile(&queue_clone, Some(link_id));
                        }
                    }

                    let ids = drain_reconcile_queue(&queue_clone, &registry_clone);
                    let has_reconciled = !ids.is_empty();
                    for id in ids {
                        if let Err(e) = reconciler.reconcile_link(id) {
                            error!("reconcile error: {e}");
                        }
                    }

                    // Dynamically synchronize watcher paths whenever links were reconciled or periodically
                    if has_reconciled || sync_check_counter.is_multiple_of(4) {
                        if let Ok(reg) = registry_clone.load() {
                            let desired = watch_paths_for_links(&reg.links);
                            watcher.sync_paths(&desired);

                            // Periodic self-healing: verify active links have their target and marker intact
                            for link in &reg.links {
                                if link.last_sync_status == linkd_core::LinkSyncStatus::Paused
                                    || link.last_sync_status == linkd_core::LinkSyncStatus::Syncing
                                {
                                    continue;
                                }
                                if !link.consumer_root.exists() || !link.source_path.exists() {
                                    continue;
                                }
                                let marker_path =
                                    link.sync_target.join(linkd_core::LinkMarker::FILE_NAME);
                                if !link.sync_target.exists() || !marker_path.exists() {
                                    enqueue_reconcile(&queue_clone, Some(link.id));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn registry(&self) -> &RegistryStore {
        &self.registry
    }

    pub fn reconciler(&self) -> &Reconciler {
        &self.reconciler
    }

    pub fn pm_hint_for_consumer(&self, consumer: &std::path::Path) -> Option<String> {
        pm_install_hint(consumer)
    }
}

fn matching_link_ids_for_event(
    paths: &[std::path::PathBuf],
    links: &[linkd_core::LinkEntry],
) -> Vec<uuid::Uuid> {
    let mut matched_ids = Vec::new();
    for path in paths {
        let clean_evt_path = linkd_core::normalize_path(path);
        for link in links {
            if link.last_sync_status == linkd_core::LinkSyncStatus::Paused {
                continue;
            }
            let clean_src = linkd_core::normalize_path(&link.source_path);
            let markers = completion_markers_for_link(link);
            let clean_consumer = linkd_core::normalize_path(&link.consumer_root);
            let clean_target = linkd_core::normalize_path(&link.sync_target);

            let is_source_match = clean_evt_path.starts_with(&clean_src);
            let is_marker_match = markers.iter().any(|m| {
                let clean_m = linkd_core::normalize_path(m);
                clean_evt_path == clean_m || clean_evt_path.starts_with(&clean_m)
            }) || clean_evt_path == clean_consumer
                || clean_evt_path
                    .parent()
                    .map(|p| p == clean_consumer)
                    .unwrap_or(false);

            // Target match: ONLY consider target events if target or marker is actually missing.
            // If target and marker exist, the event was generated by normal daemon writes or reads,
            // so ignore to avoid infinite sync loops.
            let is_target_damage_match = (clean_evt_path == clean_target
                || clean_evt_path.starts_with(&clean_target)
                || clean_target.starts_with(&clean_evt_path))
                && link.consumer_root.exists()
                && link.source_path.exists()
                && (!link.sync_target.exists()
                    || !link.sync_target.join(linkd_core::LinkMarker::FILE_NAME).exists());

            if (is_source_match || is_marker_match || is_target_damage_match)
                && !matched_ids.contains(&link.id)
            {
                matched_ids.push(link.id);
            }
        }
    }
    matched_ids
}

fn watch_paths_for_links(links: &[linkd_core::LinkEntry]) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    for link in links {
        // Watch consumer root and completion markers so we detect reinstalls & lockfile changes.
        paths.push(link.consumer_root.clone());
        paths.extend(completion_markers_for_link(link));
        // Watch the source package so we sync on code changes.
        paths.push(link.source_path.clone());
        // NOTE: do NOT watch sync_target — the daemon writes there itself,
        // which would cause an infinite reconcile loop.
    }
    paths.sort();
    paths.dedup();
    paths
}
