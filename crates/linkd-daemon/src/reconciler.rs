use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use linkd_adapters_npm::{ensure_shadow_isolation, PnpmStoreDetector};
use linkd_core::{
    LinkEntry, LinkMarker, LinkSyncStatus, LinkdError, LinkdResult,
};
use linkd_pack::{list_pack_files_cached, list_pack_files_fallback};
use linkd_registry::{Registry, RegistryStore};
use linkd_sync::SyncEngine;
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct Reconciler {
    registry: RegistryStore,
}

impl Reconciler {
    pub fn new(registry: RegistryStore) -> Self {
        Self { registry }
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

        self.registry.update_link(link_id, |entry| {
            entry.last_sync_status = LinkSyncStatus::Syncing;
        })?;

        match self.sync_link(&link) {
            Ok((hash, count, target, isolation)) => {
                self.registry.update_link(link_id, |entry| {
                    Registry::update_sync(entry, hash, count, Utc::now());
                    entry.sync_target = target;
                    entry.isolation_mode = isolation;
                })?;
                info!("synced {} ({} files)", link.package_name, count);
                Ok(())
            }
            Err(e) => {
                self.registry.update_link(link_id, |entry| {
                    entry.last_sync_status = LinkSyncStatus::Error;
                })?;
                Err(e)
            }
        }
    }

    fn sync_link(&self, link: &LinkEntry) -> LinkdResult<(String, u32, PathBuf, linkd_core::IsolationMode)> {
        let resolved = PnpmStoreDetector::resolve(&link.consumer_root, &link.package_name)?;
        ensure_shadow_isolation(&link.consumer_root, &link.package_name, &resolved)?;

        let allowlist =
            PnpmStoreDetector::build_allowlist(&link.consumer_root, resolved.forbidden_roots.clone());
        PnpmStoreDetector::assert_never_writes_global_store(&resolved.sync_target)?;

        let files = match list_pack_files_cached(&link.source_path) {
            Ok(f) => f,
            Err(_) => list_pack_files_fallback(&link.source_path)?,
        };

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
            output.sync_target,
            output.isolation_mode,
        ))
    }

    pub fn needs_reconcile(&self, link: &LinkEntry) -> LinkdResult<bool> {
        let marker = LinkMarker::read(&link.sync_target).map_err(|e| LinkdError::io(&link.sync_target, e))?;
        if marker.is_none() {
            return Ok(true);
        }

        let files = list_pack_files_fallback(&link.source_path)?;
        let hash = linkd_core::content_hash(&link.source_path, &files);
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
