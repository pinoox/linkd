use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ecosystem {
    Npm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStrategy {
    Reflink,
    Copy,
    Hardlink,
    Symlink,
}

impl Default for SyncStrategy {
    fn default() -> Self {
        Self::Reflink
    }
}

impl SyncStrategy {
    pub fn from_cli_flags(hardlink: bool, symlink: bool, copy: bool) -> Self {
        if hardlink {
            Self::Hardlink
        } else if symlink {
            Self::Symlink
        } else if copy {
            Self::Copy
        } else {
            Self::Reflink
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IsolationMode {
    ProjectLocal,
    Shadow,
}

impl Default for IsolationMode {
    fn default() -> Self {
        Self::ProjectLocal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMarker {
    pub link_id: Uuid,
    pub source_hash: String,
    pub synced_at: DateTime<Utc>,
    pub strategy: SyncStrategy,
    pub isolation_mode: IsolationMode,
}

impl LinkMarker {
    pub const FILE_NAME: &'static str = ".linkd-marker.json";

    pub fn write(&self, dir: &Path) -> std::io::Result<()> {
        let path = dir.join(Self::FILE_NAME);
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    pub fn read(dir: &Path) -> std::io::Result<Option<Self>> {
        let path = dir.join(Self::FILE_NAME);
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&data).map_err(std::io::Error::other)?))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEntry {
    pub id: Uuid,
    pub package_name: String,
    pub source_path: PathBuf,
    pub consumer_root: PathBuf,
    pub ecosystem: Ecosystem,
    pub strategy: SyncStrategy,
    pub isolation_mode: IsolationMode,
    pub sync_target: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_hash: Option<String>,
    pub last_sync_status: LinkSyncStatus,
    pub file_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkSyncStatus {
    Synced,
    Syncing,
    Pending,
    Error,
}

impl Default for LinkSyncStatus {
    fn default() -> Self {
        Self::Pending
    }
}

pub fn hash_files(source_root: &Path, relative_files: &[PathBuf]) -> String {
    let mut hasher = Sha256::new();
    let mut paths: Vec<_> = relative_files.to_vec();
    paths.sort();

    for rel in paths {
        hasher.update(rel.to_string_lossy().as_bytes());
        let full = source_root.join(rel);
        if full.is_file() {
            if let Ok(bytes) = std::fs::read(&full) {
                hasher.update(&bytes);
            }
        }
    }

    format!("sha256:{:x}", hasher.finalize())
}
