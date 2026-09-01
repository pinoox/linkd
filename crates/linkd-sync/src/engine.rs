use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use linkd_core::{content_hash, IsolationMode, LinkMarker, LinkdError, LinkdResult, SyncStrategy};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::strategy::{copy_file_with_strategy, is_file_identical};
use crate::write_guard::{WriteAllowlist, WriteGuard};

#[derive(Debug, Clone)]
pub struct SyncOutput {
    pub hash: String,
    pub file_count: u32,
    pub files_copied: u32,
    pub files_skipped: u32,
    pub files_deleted: u32,
    pub sync_target: PathBuf,
    pub isolation_mode: IsolationMode,
}

pub struct SyncEngine {
    allowlist: WriteAllowlist,
}

impl SyncEngine {
    pub fn new(allowlist: WriteAllowlist) -> Self {
        Self { allowlist }
    }

    /// Performs smart, content-aware incremental sync from source_root into sync_target.
    ///
    /// 1. Updates new/modified files in-place with timestamp preservation (Layer 1 & 2 gate).
    /// 2. Skips identical files completely to avoid file-locks and rebuild loops.
    /// 3. Removes stale/deleted files from sync_target.
    /// 4. Writes/updates .linkd-marker.json.
    pub fn sync(
        &self,
        link_id: Uuid,
        source_root: &Path,
        sync_target: &Path,
        files: &[PathBuf],
        strategy: SyncStrategy,
        isolation_mode: IsolationMode,
    ) -> LinkdResult<SyncOutput> {
        WriteGuard::new(&self.allowlist).check(sync_target)?;

        let hash = content_hash(source_root, files);

        if let Some(parent) = sync_target.parent() {
            fs::create_dir_all(parent).map_err(|e| LinkdError::io(parent, e))?;
        }
        fs::create_dir_all(sync_target).map_err(|e| LinkdError::io(sync_target, e))?;

        let mut files_copied = 0u32;
        let mut files_skipped = 0u32;
        let mut files_deleted = 0u32;

        let mut target_file_set = HashSet::new();

        // 1. Sync new and modified files
        for rel in files {
            let src = source_root.join(rel);
            let dst = sync_target.join(rel);

            target_file_set.insert(rel.clone());

            if src.is_dir() {
                fs::create_dir_all(&dst).map_err(|e| LinkdError::io(&dst, e))?;
                continue;
            }

            if src.is_file() {
                if is_file_identical(&src, &dst) {
                    files_skipped += 1;
                } else {
                    copy_file_with_strategy(&src, &dst, strategy)
                        .map_err(|e| LinkdError::io(&dst, e))?;
                    files_copied += 1;
                }
            }
        }

        // 2. Cleanup stale / deleted files in sync_target
        if sync_target.is_dir() {
            for entry in WalkDir::new(sync_target)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    if file_name == ".linkd-marker.json" {
                        continue;
                    }
                    if let Ok(rel) = path.strip_prefix(sync_target) {
                        let rel_buf = rel.to_path_buf();
                        if !target_file_set.contains(&rel_buf) {
                            let _ = fs::remove_file(path);
                            files_deleted += 1;
                        }
                    }
                }
            }
        }

        // 3. Write / update marker
        let marker = LinkMarker {
            link_id,
            source_hash: hash.clone(),
            synced_at: Utc::now(),
            strategy,
            isolation_mode,
        };
        marker
            .write(sync_target)
            .map_err(|e| LinkdError::io(sync_target, e))?;

        let total_files = files_copied + files_skipped;

        Ok(SyncOutput {
            hash,
            file_count: total_files,
            files_copied,
            files_skipped,
            files_deleted,
            sync_target: sync_target.to_path_buf(),
            isolation_mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn incremental_sync_skips_unchanged_files() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let target = consumer.join("node_modules").join("pkg");

        let allowlist = WriteAllowlist::from_consumer(&consumer, vec![]);
        let engine = SyncEngine::new(allowlist);

        let source = tmp.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.js"), b"console.log('a');").unwrap();
        fs::write(source.join("b.js"), b"console.log('b');").unwrap();

        let files = vec![PathBuf::from("a.js"), PathBuf::from("b.js")];

        // First sync: all files copied
        let out1 = engine
            .sync(
                Uuid::new_v4(),
                &source,
                &target,
                &files,
                SyncStrategy::Copy,
                IsolationMode::ProjectLocal,
            )
            .unwrap();

        assert_eq!(out1.files_copied, 2);
        assert_eq!(out1.files_skipped, 0);

        // Second sync without changes: all files skipped
        let out2 = engine
            .sync(
                Uuid::new_v4(),
                &source,
                &target,
                &files,
                SyncStrategy::Copy,
                IsolationMode::ProjectLocal,
            )
            .unwrap();

        assert_eq!(out2.files_copied, 0);
        assert_eq!(out2.files_skipped, 2);
    }

    #[test]
    fn incremental_sync_removes_stale_files() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let target = consumer.join("node_modules").join("pkg");

        let allowlist = WriteAllowlist::from_consumer(&consumer, vec![]);
        let engine = SyncEngine::new(allowlist);

        let source = tmp.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.js"), b"a").unwrap();
        fs::write(source.join("b.js"), b"b").unwrap();

        let files = vec![PathBuf::from("a.js"), PathBuf::from("b.js")];
        engine
            .sync(
                Uuid::new_v4(),
                &source,
                &target,
                &files,
                SyncStrategy::Copy,
                IsolationMode::ProjectLocal,
            )
            .unwrap();

        assert!(target.join("b.js").exists());

        // Now remove b.js from source files list
        let files_after = vec![PathBuf::from("a.js")];
        let out = engine
            .sync(
                Uuid::new_v4(),
                &source,
                &target,
                &files_after,
                SyncStrategy::Copy,
                IsolationMode::ProjectLocal,
            )
            .unwrap();

        assert_eq!(out.files_deleted, 1);
        assert_eq!(out.files_skipped, 1);
        assert!(!target.join("b.js").exists());
        assert!(target.join("a.js").exists());
    }
}
