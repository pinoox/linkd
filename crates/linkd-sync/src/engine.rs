use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use linkd_core::{
    content_hash, tmp_dir, IsolationMode, LinkMarker, LinkdError, LinkdResult, SyncStrategy,
};
use uuid::Uuid;

use crate::strategy::mirror_tree;
use crate::write_guard::{WriteAllowlist, WriteGuard};

#[derive(Debug, Clone)]
pub struct SyncOutput {
    pub hash: String,
    pub file_count: u32,
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
        let tmp = self.prepare_tmp_dir(link_id)?;
        let file_count = mirror_tree(source_root, &tmp, files, strategy)
            .map_err(|e| LinkdError::io(&tmp, e))?;

        let marker = LinkMarker {
            link_id,
            source_hash: hash.clone(),
            synced_at: Utc::now(),
            strategy,
            isolation_mode,
        };
        marker
            .write(&tmp)
            .map_err(|e| LinkdError::io(&tmp, e))?;

        self.atomic_swap(&tmp, sync_target)?;

        Ok(SyncOutput {
            hash,
            file_count,
            sync_target: sync_target.to_path_buf(),
            isolation_mode,
        })
    }

    fn prepare_tmp_dir(&self, link_id: Uuid) -> LinkdResult<PathBuf> {
        let base = tmp_dir();
        fs::create_dir_all(&base).map_err(|e| LinkdError::io(&base, e))?;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let tmp = base.join(format!("{link_id}-{ts}"));
        if tmp.exists() {
            let _ = fs::remove_dir_all(&tmp);
        }
        fs::create_dir_all(&tmp).map_err(|e| LinkdError::io(&tmp, e))?;
        Ok(tmp)
    }

    fn atomic_swap(&self, tmp: &Path, target: &Path) -> LinkdResult<()> {
        WriteGuard::new(&self.allowlist).check(target)?;

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| LinkdError::io(parent, e))?;
        }

        if !target.exists() {
            fs::rename(tmp, target).map_err(|e| LinkdError::io(target, e))?;
            return Ok(());
        }

        // Windows cannot rename a directory over an existing one; keep the target path
        // present by replacing contents in-place (ADR-002 invariant).
        #[cfg(windows)]
        if target.is_dir() {
            replace_dir_in_place(tmp, target)?;
            return Ok(());
        }

        // Unix directory/file swap via sibling rename (atomic on same volume).
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let backup = target.with_file_name(format!(
            "{}.old-{}",
            target.file_name().unwrap_or_default().to_string_lossy(),
            ts
        ));

        if backup.exists() {
            remove_path_all(&backup).map_err(|e| LinkdError::io(&backup, e))?;
        }

        fs::rename(target, &backup).map_err(|e| LinkdError::io(target, e))?;
        match fs::rename(tmp, target) {
            Ok(()) => {
                let _ = remove_path_all(&backup);
                Ok(())
            }
            Err(e) => {
                let _ = fs::rename(&backup, target);
                Err(LinkdError::io(target, e))
            }
        }
    }
}

fn remove_path_all(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn replace_dir_in_place(src: &Path, dst: &Path) -> LinkdResult<()> {
    fs::create_dir_all(dst).map_err(|e| LinkdError::io(dst, e))?;

    for entry in fs::read_dir(dst).map_err(|e| LinkdError::io(dst, e))? {
        let entry = entry.map_err(|e| LinkdError::io(dst, e))?;
        remove_path_all(&entry.path()).map_err(|e| LinkdError::io(entry.path(), e))?;
    }

    for entry in fs::read_dir(src).map_err(|e| LinkdError::io(src, e))? {
        let entry = entry.map_err(|e| LinkdError::io(src, e))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_all(&from, &to).map_err(|e| LinkdError::io(&to, e))?;
        } else {
            fs::copy(&from, &to).map_err(|e| LinkdError::io(&to, e))?;
        }
    }

    remove_path_all(src).map_err(|e| LinkdError::io(src, e))?;
    Ok(())
}

fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = to.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn atomic_swap_never_leaves_target_missing() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let target = consumer.join("node_modules").join("pkg");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("old.js"), b"old").unwrap();

        let allowlist = WriteAllowlist::from_consumer(&consumer, vec![]);
        let engine = SyncEngine::new(allowlist);

        let source = tmp.path().join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("index.js"), b"new").unwrap();

        let files = vec![PathBuf::from("index.js")];
        let out = engine
            .sync(
                Uuid::new_v4(),
                &source,
                &target,
                &files,
                SyncStrategy::Copy,
                IsolationMode::ProjectLocal,
            )
            .unwrap();

        assert_eq!(out.file_count, 1);
        assert!(target.join("index.js").exists());
        assert!(LinkMarker::read(&target).unwrap().is_some());
    }
}
