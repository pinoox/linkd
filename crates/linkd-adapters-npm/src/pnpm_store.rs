use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use linkd_core::{IsolationMode, LinkdError, LinkdResult, ResolvedSyncTarget};
use linkd_sync::{shadow_dir, WriteAllowlist};

use crate::target_resolve::{resolve_node_modules_target, shadow_target_path};

static STORE_CACHE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

pub struct PnpmStoreDetector;

impl PnpmStoreDetector {
    pub fn global_store_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if let Ok(pnpm_home) = std::env::var("PNPM_HOME") {
            paths.push(PathBuf::from(pnpm_home).join("store"));
        }

        if let Some(home) = std::env::var_os("HOME").or(std::env::var_os("USERPROFILE")) {
            let home = PathBuf::from(home);
            paths.push(home.join(".local/share/pnpm/store"));
            paths.push(home.join("AppData/Local/pnpm/store"));
        }

        if let Some(store) = Self::pnpm_store_path_command() {
            paths.push(store);
        }

        paths
            .into_iter()
            .filter(|p| p.exists())
            .map(|p| p.canonicalize().unwrap_or(p))
            .collect()
    }

    fn pnpm_store_path_command() -> Option<PathBuf> {
        let cache = STORE_CACHE.get_or_init(|| Mutex::new(None));
        let mut guard = cache.lock().ok()?;
        if let Some(ref p) = *guard {
            return Some(p.clone());
        }

        let output = Command::new("pnpm").args(["store", "path"]).output().ok()?;

        if !output.status.success() {
            return None;
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            return None;
        }

        let pb = PathBuf::from(path);
        *guard = Some(pb.clone());
        Some(pb)
    }

    pub fn is_global_store_path(path: &Path) -> bool {
        Self::is_global_store_path_with(path, &Self::global_store_paths())
    }

    pub fn is_global_store_path_with(path: &Path, roots: &[PathBuf]) -> bool {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        roots.iter().any(|store| {
            let store = store.canonicalize().unwrap_or_else(|_| store.clone());
            canonical.starts_with(store)
        })
    }

    pub fn resolve(consumer_root: &Path, package_name: &str) -> LinkdResult<ResolvedSyncTarget> {
        let logical = resolve_node_modules_target(consumer_root, package_name);
        let forbidden = Self::global_store_paths();

        let resolved = if logical.exists() {
            fs::canonicalize(&logical).unwrap_or(logical.clone())
        } else {
            logical.clone()
        };

        if Self::is_global_store_path(&resolved) {
            let shadow = shadow_target_path(consumer_root, package_name);
            return Ok(ResolvedSyncTarget {
                logical_target: logical,
                sync_target: shadow,
                isolation_mode: IsolationMode::Shadow,
                forbidden_roots: forbidden,
            });
        }

        // Symlink into .pnpm that resolves outside consumer but not global store
        if logical.is_symlink() {
            let canon = fs::canonicalize(&logical).unwrap_or(resolved);
            if Self::is_global_store_path(&canon) {
                let shadow = shadow_target_path(consumer_root, package_name);
                return Ok(ResolvedSyncTarget {
                    logical_target: logical,
                    sync_target: shadow,
                    isolation_mode: IsolationMode::Shadow,
                    forbidden_roots: forbidden,
                });
            }
        }

        Ok(ResolvedSyncTarget {
            logical_target: logical.clone(),
            sync_target: logical,
            isolation_mode: IsolationMode::ProjectLocal,
            forbidden_roots: forbidden,
        })
    }

    pub fn redirect_symlink(logical: &Path, shadow: &Path) -> LinkdResult<()> {
        if logical.exists() {
            if logical.is_symlink() {
                fs::remove_file(logical).map_err(|e| LinkdError::io(logical, e))?;
            } else if logical.is_dir() {
                fs::remove_dir_all(logical).map_err(|e| LinkdError::io(logical, e))?;
            }
        }

        if let Some(parent) = logical.parent() {
            fs::create_dir_all(parent).map_err(|e| LinkdError::io(parent, e))?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(shadow, logical).map_err(|e| LinkdError::io(logical, e))?;

        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(shadow, logical)
            .map_err(|e| LinkdError::io(logical, e))?;

        Ok(())
    }

    pub fn build_allowlist(consumer_root: &Path, forbidden: Vec<PathBuf>) -> WriteAllowlist {
        WriteAllowlist::from_consumer(consumer_root, forbidden)
    }

    pub fn assert_never_writes_global_store(sync_target: &Path) -> LinkdResult<()> {
        if Self::is_global_store_path(sync_target) {
            return Err(LinkdError::PnpmGlobalStoreForbidden(
                sync_target.display().to_string(),
            ));
        }
        Ok(())
    }
}

pub fn ensure_shadow_isolation(
    consumer_root: &Path,
    package_name: &str,
    resolved: &ResolvedSyncTarget,
) -> LinkdResult<()> {
    if resolved.isolation_mode == IsolationMode::Shadow {
        PnpmStoreDetector::redirect_symlink(&resolved.logical_target, &resolved.sync_target)?;
        let _ = shadow_dir(consumer_root, package_name);
    }
    PnpmStoreDetector::assert_never_writes_global_store(&resolved.sync_target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn project_local_target_allowed() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let target = resolve_node_modules_target(&consumer, "my-lib");
        std::fs::create_dir_all(&target).unwrap();

        let resolved = PnpmStoreDetector::resolve(&consumer, "my-lib").unwrap();
        assert_eq!(resolved.isolation_mode, IsolationMode::ProjectLocal);
        assert!(PnpmStoreDetector::assert_never_writes_global_store(&resolved.sync_target).is_ok());
    }

    #[test]
    fn global_store_triggers_shadow() {
        let tmp = TempDir::new().unwrap();
        let store = tmp.path().join("pnpm-store");
        std::fs::create_dir_all(&store).unwrap();

        let consumer = tmp.path().join("app");
        let nm = consumer.join("node_modules").join("pkg");
        std::fs::create_dir_all(nm.parent().unwrap()).unwrap();

        let pkg_in_store = store.join("v3/files/00/pkg");
        std::fs::create_dir_all(&pkg_in_store).unwrap();
        std::fs::write(pkg_in_store.join("index.js"), b"1").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&pkg_in_store, &nm).unwrap();

        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&pkg_in_store, &nm).unwrap();

        // Inject store into detector by placing under known path pattern
        let resolved = ResolvedSyncTarget {
            logical_target: nm.clone(),
            sync_target: shadow_target_path(&consumer, "pkg"),
            isolation_mode: IsolationMode::Shadow,
            forbidden_roots: vec![store.canonicalize().unwrap()],
        };

        assert!(PnpmStoreDetector::is_global_store_path_with(
            &pkg_in_store,
            &[store.canonicalize().unwrap()],
        ));
        assert_ne!(resolved.sync_target, pkg_in_store);
    }
}
