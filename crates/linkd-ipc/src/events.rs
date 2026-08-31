use std::path::PathBuf;

use chrono::{DateTime, Utc};
use linkd_core::LinkSyncStatus;
use serde::{Deserialize, Serialize};

use crate::protocol::LinkStatusSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonEvent {
    LinkStatusChanged {
        package_name: String,
        source: PathBuf,
        status: LinkSyncStatus,
        last_synced_at: Option<DateTime<Utc>>,
    },
    SyncStarted {
        package_name: String,
        source: PathBuf,
        target: PathBuf,
        files_count: usize,
    },
    SyncCompleted {
        package_name: String,
        source: PathBuf,
        target: PathBuf,
        duration_ms: u64,
        files_synced: usize,
    },
    SyncFailed {
        package_name: String,
        source: PathBuf,
        error: String,
    },
    LogMessage {
        timestamp: DateTime<Utc>,
        level: String,
        ecosystem: Option<String>,
        message: String,
    },
    Snapshot {
        snapshot: LinkStatusSnapshot,
    },
}
