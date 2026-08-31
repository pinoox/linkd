use std::path::PathBuf;

use chrono::{DateTime, Utc};
use linkd_core::{Ecosystem, IsolationMode, LinkEntry, LinkSyncStatus, SyncStrategy};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const REGISTRY_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryFile {
    pub version: u32,
    pub links: Vec<LinkEntry>,
}

impl Default for RegistryFile {
    fn default() -> Self {
        Self {
            version: REGISTRY_VERSION,
            links: Vec::new(),
        }
    }
}

pub struct Registry;

impl Registry {
    pub fn new_link(
        package_name: String,
        source_path: PathBuf,
        consumer_root: PathBuf,
        sync_target: PathBuf,
        strategy: SyncStrategy,
        isolation_mode: IsolationMode,
    ) -> LinkEntry {
        LinkEntry {
            id: Uuid::new_v4(),
            package_name,
            source_path,
            consumer_root,
            ecosystem: Ecosystem::Npm,
            strategy,
            isolation_mode,
            sync_target,
            created_at: Utc::now(),
            last_sync_at: None,
            last_sync_hash: None,
            last_sync_status: LinkSyncStatus::Pending,
            file_count: 0,
        }
    }

    pub fn find_by_id(links: &[LinkEntry], id: Uuid) -> Option<&LinkEntry> {
        links.iter().find(|l| l.id == id)
    }

    pub fn find_by_id_mut(links: &mut [LinkEntry], id: Uuid) -> Option<&mut LinkEntry> {
        links.iter_mut().find(|l| l.id == id)
    }

    pub fn find_by_package<'a>(links: &'a [LinkEntry], name: &str) -> Option<&'a LinkEntry> {
        links.iter().find(|l| l.package_name == name)
    }

    pub fn find_by_source<'a>(links: &'a [LinkEntry], source: &PathBuf) -> Option<&'a LinkEntry> {
        links.iter().find(|l| &l.source_path == source)
    }

    pub fn remove_by_package(links: &mut Vec<LinkEntry>, name: &str) -> Option<LinkEntry> {
        links.iter()
            .position(|l| l.package_name == name)
            .map(|idx| links.remove(idx))
    }

    pub fn update_sync(
        entry: &mut LinkEntry,
        hash: String,
        file_count: u32,
        at: DateTime<Utc>,
    ) {
        entry.last_sync_hash = Some(hash);
        entry.last_sync_at = Some(at);
        entry.last_sync_status = LinkSyncStatus::Synced;
        entry.file_count = file_count;
    }
}
