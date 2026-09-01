use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use linkd_adapters::{
    build_allowlist_for_link, ensure_isolation, list_files_for_link, post_sync_hint_for_link,
    resolve_for_reconcile,
};
use linkd_core::{content_hash, LinkEntry, LinkMarker, LinkSyncStatus, LinkdError, LinkdResult};
use linkd_registry::{Registry, RegistryStore};
use linkd_sync::SyncEngine;
use tracing::{error, info, warn};
use uuid::Uuid;

use linkd_ipc::DaemonEvent;
use tokio::sync::broadcast;

pub struct Reconciler {
    registry: RegistryStore,
    events_tx: Option<broadcast::Sender<DaemonEvent>>,
}

impl Reconciler {
    pub fn new(registry: RegistryStore) -> Self {
        Self {
            registry,
            events_tx: None,
        }
    }

    pub fn with_events_tx(mut self, tx: broadcast::Sender<DaemonEvent>) -> Self {
        self.events_tx = Some(tx);
        self
    }

    pub fn reconcile_all(&self) -> LinkdResult<()> {
        let registry = self.registry.load()?;
        for link in registry.links {
            if let Err(e) = self.reconcile_link(link.id) {
                error!("reconcile failed for {}: {e}", link.package_name);
            }
        }
        Ok(())
    }

    pub fn reconcile_link(&self, link_id: Uuid) -> LinkdResult<()> {
        let registry = self.registry.load()?;
        let link = Registry::find_by_id(&registry.links, link_id)
            .ok_or_else(|| LinkdError::PackageNotFound(link_id.to_string()))?
            .clone();

        if link.last_sync_status == LinkSyncStatus::Paused {
            info!("skipping sync for paused link {}", link.package_name);
            return Ok(());
        }

        let start_time = std::time::Instant::now();

        self.registry.update_link(link_id, |entry| {
            entry.last_sync_status = LinkSyncStatus::Syncing;
        })?;

        if let Some(events_tx) = &self.events_tx {
            let _ = events_tx.send(DaemonEvent::SyncStarted {
                package_name: link.package_name.clone(),
                source: link.source_path.clone(),
                target: link.sync_target.clone(),
                files_count: link.file_count as usize,
            });
            let _ = events_tx.send(DaemonEvent::LinkStatusChanged {
                package_name: link.package_name.clone(),
                source: link.source_path.clone(),
                status: LinkSyncStatus::Syncing,
                last_synced_at: link.last_sync_at,
            });
        }

        match self.sync_link(&link) {
            Ok((hash, count, copied, skipped, deleted, target, isolation)) => {
                let now = Utc::now();
                self.registry.update_link(link_id, |entry| {
                    Registry::update_sync(entry, hash, count, now);
                    entry.sync_target = target.clone();
                    entry.isolation_mode = isolation;
                })?;

                let duration_ms = start_time.elapsed().as_millis() as u64;

                let sync_summary = if copied > 0 || deleted > 0 {
                    format!("{copied} updated, {deleted} deleted, {skipped} unchanged")
                } else {
                    format!("up-to-date, {skipped} unchanged")
                };

                if let Some(events_tx) = &self.events_tx {
                    let _ = events_tx.send(DaemonEvent::SyncCompleted {
                        package_name: link.package_name.clone(),
                        source: link.source_path.clone(),
                        target: target.clone(),
                        duration_ms,
                        files_synced: count as usize,
                    });
                    let _ = events_tx.send(DaemonEvent::LinkStatusChanged {
                        package_name: link.package_name.clone(),
                        source: link.source_path.clone(),
                        status: LinkSyncStatus::Synced,
                        last_synced_at: Some(now),
                    });
                    let _ = events_tx.send(DaemonEvent::LogMessage {
                        timestamp: now,
                        level: "INFO".into(),
                        ecosystem: Some(format!("{:?}", link.ecosystem)),
                        message: format!(
                            "synced {count} files ({sync_summary} in {duration_ms}ms) -> {}",
                            target.display()
                        ),
                    });
                }

                if let Some(hint) = post_sync_hint_for_link(&link) {
                    info!("hint for {}: {hint}", link.package_name);
                    if let Some(events_tx) = &self.events_tx {
                        let _ = events_tx.send(DaemonEvent::LogMessage {
                            timestamp: Utc::now(),
                            level: "HINT".into(),
                            ecosystem: Some(format!("{:?}", link.ecosystem)),
                            message: hint,
                        });
                    }
                }
                info!(
                    "synced {} ({count} files, {sync_summary} in {duration_ms}ms)",
                    link.package_name
                );
                Ok(())
            }
            Err(e) => {
                self.registry.update_link(link_id, |entry| {
                    entry.last_sync_status = LinkSyncStatus::Error;
                })?;

                if let Some(events_tx) = &self.events_tx {
                    let _ = events_tx.send(DaemonEvent::SyncFailed {
                        package_name: link.package_name.clone(),
                        source: link.source_path.clone(),
                        error: e.to_string(),
                    });
                    let _ = events_tx.send(DaemonEvent::LinkStatusChanged {
                        package_name: link.package_name.clone(),
                        source: link.source_path.clone(),
                        status: LinkSyncStatus::Error,
                        last_synced_at: link.last_sync_at,
                    });
                    let _ = events_tx.send(DaemonEvent::LogMessage {
                        timestamp: Utc::now(),
                        level: "ERROR".into(),
                        ecosystem: Some(format!("{:?}", link.ecosystem)),
                        message: format!("sync failed: {e}"),
                    });
                }
                Err(e)
            }
        }
    }

    fn sync_link(
        &self,
        link: &LinkEntry,
    ) -> LinkdResult<(
        String,
        u32,
        u32,
        u32,
        u32,
        PathBuf,
        linkd_core::IsolationMode,
    )> {
        let resolved = resolve_for_reconcile(link)?;
        ensure_isolation(link, &resolved)?;

        let allowlist = build_allowlist_for_link(link, &resolved);
        let files = list_files_for_link(link)?;

        let engine = SyncEngine::new(allowlist);
        let output = engine.sync(
            link.id,
            &link.source_path,
            &resolved.sync_target,
            &files,
            link.strategy,
            resolved.isolation_mode,
        )?;

        if resolved.isolation_mode == linkd_core::IsolationMode::Shadow {
            warn!(
                "using shadow isolation for {} at {}",
                link.package_name,
                output.sync_target.display()
            );
        }

        Ok((
            output.hash,
            output.file_count,
            output.files_copied,
            output.files_skipped,
            output.files_deleted,
            output.sync_target,
            output.isolation_mode,
        ))
    }

    pub fn needs_reconcile(&self, link: &LinkEntry) -> LinkdResult<bool> {
        let marker = LinkMarker::read(&link.sync_target)
            .map_err(|e| LinkdError::io(&link.sync_target, e))?;
        if marker.is_none() {
            return Ok(true);
        }

        let files = list_files_for_link(link)?;
        let hash = content_hash(&link.source_path, &files);
        Ok(marker.map(|m| m.source_hash != hash).unwrap_or(true))
    }
}

pub type ReconcileQueue = Arc<Mutex<Vec<Uuid>>>;

pub fn enqueue_reconcile(queue: &ReconcileQueue, id: Option<Uuid>) {
    let mut guard = queue.lock().expect("lock");
    match id {
        Some(id) => guard.push(id),
        None => guard.push(Uuid::nil()),
    }
}

pub fn drain_reconcile_queue(queue: &ReconcileQueue, registry: &RegistryStore) -> Vec<Uuid> {
    let mut guard = queue.lock().expect("lock");
    let items = std::mem::take(&mut *guard);
    drop(guard);

    if items.iter().any(|id| *id == Uuid::nil()) {
        registry
            .load()
            .map(|r| r.links.iter().map(|l| l.id).collect())
            .unwrap_or_default()
    } else {
        items
    }
}
