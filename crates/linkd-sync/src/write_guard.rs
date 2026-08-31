use std::path::{Path, PathBuf};

use linkd_core::paths::normalize_path;
use linkd_core::{LinkdError, LinkdResult};

/// Paths that sync is allowed to write into.
#[derive(Debug, Clone)]
pub struct WriteAllowlist {
    #[allow(dead_code)]
    consumer_root: PathBuf,
    allowed_roots: Vec<PathBuf>,
    forbidden_roots: Vec<PathBuf>,
}

impl WriteAllowlist {
    pub fn new(consumer_root: PathBuf, forbidden_roots: Vec<PathBuf>) -> Self {
        let consumer = normalize_path(&consumer_root);
        let allowed_roots = vec![consumer.join("node_modules")];
        Self {
            consumer_root: consumer,
            allowed_roots,
            forbidden_roots: forbidden_roots.into_iter().map(|p| normalize_path(&p)).collect(),
        }
    }

    pub fn from_consumer(consumer_root: &Path, forbidden_roots: Vec<PathBuf>) -> Self {
        Self::from_consumer_subdirs(consumer_root, &["node_modules"], forbidden_roots)
    }

    pub fn from_consumer_subdirs(
        consumer_root: &Path,
        subdirs: &[&str],
        forbidden_roots: Vec<PathBuf>,
    ) -> Self {
        let consumer = normalize_path(consumer_root);
        let allowed_roots = subdirs.iter().map(|s| consumer.join(s)).collect();
        Self {
            consumer_root: consumer,
            allowed_roots,
            forbidden_roots: forbidden_roots
                .into_iter()
                .map(|p| normalize_path(&p))
                .collect(),
        }
    }

    pub fn from_allowed_roots(allowed_roots: Vec<PathBuf>, forbidden_roots: Vec<PathBuf>) -> Self {
        let allowed_roots = allowed_roots
            .into_iter()
            .map(|p| normalize_path(&p))
            .collect::<Vec<_>>();
        let consumer_root = allowed_roots
            .first()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            consumer_root,
            allowed_roots,
            forbidden_roots: forbidden_roots
                .into_iter()
                .map(|p| normalize_path(&p))
                .collect(),
        }
    }

    pub fn is_allowed(&self, path: &Path) -> bool {
        let canonical = normalize_path(path);

        for forbidden in &self.forbidden_roots {
            if canonical.starts_with(normalize_path(forbidden)) {
                return false;
            }
        }

        self.allowed_roots
            .iter()
            .any(|root| canonical.starts_with(normalize_path(root)))
    }

    pub fn assert_writable(&self, path: &Path) -> LinkdResult<()> {
        if self.is_allowed(path) {
            return Ok(());
        }

        let canonical = normalize_path(path);
        for forbidden in &self.forbidden_roots {
            if canonical.starts_with(normalize_path(forbidden)) {
                return Err(LinkdError::PnpmGlobalStoreForbidden(
                    path.display().to_string(),
                ));
            }
        }

        let allowed = self
            .allowed_roots
            .iter()
            .map(|p| normalize_path(p).display().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        Err(LinkdError::WriteBlocked(format!(
            "path {} is outside allowed write region ({allowed})",
            path.display(),
        )))
    }
}

pub struct WriteGuard<'a> {
    allowlist: &'a WriteAllowlist,
}

impl<'a> WriteGuard<'a> {
    pub fn new(allowlist: &'a WriteAllowlist) -> Self {
        Self { allowlist }
    }

    pub fn check(&self, path: &Path) -> LinkdResult<()> {
        self.allowlist.assert_writable(path)
    }

    pub fn check_all(&self, paths: &[PathBuf]) -> LinkdResult<()> {
        for p in paths {
            self.check(p)?;
        }
        Ok(())
    }
}

pub fn shadow_dir(consumer_root: &Path, package_name: &str) -> PathBuf {
    let mut shadow = consumer_root.join("node_modules").join(".linkd-shadow");
    for component in package_name.split('/') {
        shadow.push(component);
    }
    shadow
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn blocks_forbidden_global_store() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let nm = consumer.join("node_modules");
        std::fs::create_dir_all(&nm).unwrap();

        let global_store = tmp.path().join("pnpm-store");
        std::fs::create_dir_all(&global_store).unwrap();

        let allowlist = WriteAllowlist::from_consumer(&consumer, vec![global_store.clone()]);
        let bad = global_store.join("v3/files/00/ab/pkg/index.js");
        std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
        std::fs::write(&bad, b"x").unwrap();

        assert!(allowlist.assert_writable(&bad).is_err());
    }

    #[test]
    fn allows_shadow_path() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let shadow = shadow_dir(&consumer, "@scope/pkg");
        std::fs::create_dir_all(&shadow).unwrap();

        let allowlist = WriteAllowlist::from_consumer(&consumer, vec![]);
        assert!(allowlist.assert_writable(&shadow.join("index.js")).is_ok());
    }

    #[test]
    fn allows_vendor_path() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let vendor_pkg = consumer.join("vendor").join("acme").join("pkg");
        std::fs::create_dir_all(&vendor_pkg).unwrap();

        let allowlist = WriteAllowlist::from_consumer_subdirs(&consumer, &["vendor"], vec![]);
        assert!(allowlist
            .assert_writable(&vendor_pkg.join("src.php"))
            .is_ok());
    }
}
