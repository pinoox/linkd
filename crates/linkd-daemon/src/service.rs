use std::sync::{Arc, Mutex};
use std::time::Duration;

use linkd_ipc::{IpcServer, ReconcileHook};
use linkd_registry::RegistryStore;
use linkd_watcher::{DebouncePool, watch_marker_paths, LinkWatcher, WatchEventKind};
use tokio::sync::mpsc;
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
        linkd_core::ensure_home().map_err(|e| linkd_core::LinkdError::io(linkd_core::linkd_home(), e))?;

        let queue = self.reconcile_queue.clone();
        let registry_path = self.registry.path().to_path_buf();

        let hook: ReconcileHook = Arc::new(move |link_id| {
            enqueue_reconcile(&queue, link_id);
        });

        let ipc_registry = RegistryStore::new(registry_path.clone());
        let ipc = IpcServer::new(ipc_registry).with_reconcile_hook(hook);

        let reconciler = Reconciler::new(RegistryStore::new(registry_path.clone()));
        let queue_clone = self.reconcile_queue.clone();
        let registry_clone = RegistryStore::new(registry_path.clone());

        // Initial reconcile
        let _ = reconciler.reconcile_all();

        let registry_for_watch = self.registry.load().unwrap_or_default();
        let watch_paths = watch_marker_paths(&registry_for_watch.links);

        let (_watcher, watch_rx) = LinkWatcher::new(watch_paths).map_err(|e| {
            linkd_core::LinkdError::Other(format!("watcher failed: {e}"))
        })?;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        tokio::spawn(async move {
            if let Err(e) = ipc.run().await {
                error!("ipc server stopped: {e}");
                let _ = shutdown_tx.send(()).await;
            }
        });

        let mut debounce = DebouncePool::new(300);
        let mut interval = time::interval(Duration::from_millis(100));

        info!("linkd daemon running");

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => break,
                _ = interval.tick() => {
                    while let Ok(event) = watch_rx.try_recv() {
                        let key: String = match event.kind {
                            WatchEventKind::MarkerChanged => "marker".into(),
                            WatchEventKind::TargetChanged => "target".into(),
                            WatchEventKind::SourceChanged => "source".into(),
                        };
                        debounce.push(key, event.path);
                    }

                    for evt in debounce.ready() {
                        info!("debounced event {:?} paths={}", evt.key, evt.paths.len());
                        enqueue_reconcile(&queue_clone, None);
                    }

                    let ids = drain_reconcile_queue(&queue_clone, &registry_clone);
                    for id in ids {
                        if let Err(e) = reconciler.reconcile_link(id) {
                            error!("reconcile error: {e}");
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
