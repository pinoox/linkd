use std::fs::{File, OpenOptions};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};

use fd_lock::RwLock;
use linkd_core::{registry_path, LinkdError, LinkdResult};
use uuid::Uuid;

use crate::schema::{Registry, RegistryFile, REGISTRY_VERSION};

#[derive(Clone, Debug)]
pub struct RegistryStore {
    path: PathBuf,
}

impl Default for RegistryStore {
    fn default() -> Self {
        Self::new(registry_path())
    }
}

impl RegistryStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn open_locked(&self) -> LinkdResult<RwLock<File>> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| LinkdError::io(parent, e))?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
            .map_err(|e| LinkdError::io(&self.path, e))?;

        Ok(RwLock::new(file))
    }

    pub fn load(&self) -> LinkdResult<RegistryFile> {
        let lock = self.open_locked()?;
        let _guard = lock
            .read()
            .map_err(|e| LinkdError::Registry(e.to_string()))?;
        let contents =
            std::fs::read_to_string(&self.path).map_err(|e| LinkdError::io(&self.path, e))?;

        if contents.trim().is_empty() {
            return Ok(RegistryFile::default());
        }

        let registry: RegistryFile =
            serde_json::from_str(&contents).map_err(|e| LinkdError::Registry(e.to_string()))?;
        Ok(registry.migrate())
    }

    pub fn save(&self, registry: &RegistryFile) -> LinkdResult<()> {
        if registry.version != REGISTRY_VERSION {
            return Err(LinkdError::Registry(format!(
                "unsupported registry version {}",
                registry.version
            )));
        }

        let mut lock = self.open_locked()?;
        let mut guard = lock
            .write()
            .map_err(|e| LinkdError::Registry(e.to_string()))?;

        let json = serde_json::to_string_pretty(registry)
            .map_err(|e| LinkdError::Registry(e.to_string()))?;
        guard
            .set_len(0)
            .map_err(|e| LinkdError::io(&self.path, e))?;
        guard
            .seek(std::io::SeekFrom::Start(0))
            .map_err(|e| LinkdError::io(&self.path, e))?;
        guard
            .write_all(json.as_bytes())
            .map_err(|e| LinkdError::io(&self.path, e))?;
        guard
            .sync_all()
            .map_err(|e| LinkdError::io(&self.path, e))?;
        Ok(())
    }

    pub fn with_mut<F, T>(&self, f: F) -> LinkdResult<T>
    where
        F: FnOnce(&mut RegistryFile) -> LinkdResult<T>,
    {
        let mut registry = self.load()?;
        let result = f(&mut registry)?;
        self.save(&registry)?;
        Ok(result)
    }

    pub fn add_link(&self, mut entry: linkd_core::LinkEntry) -> LinkdResult<linkd_core::LinkEntry> {
        entry.source_path = linkd_core::clean_path(&entry.source_path);
        entry.consumer_root = linkd_core::clean_path(&entry.consumer_root);
        entry.sync_target = linkd_core::clean_path(&entry.sync_target);
        if let Some(ct) = &mut entry.custom_target {
            *ct = linkd_core::clean_path(ct);
        }

        self.with_mut(|reg| {
            if let Some(existing) = reg.links.iter_mut().find(|l| {
                (linkd_core::clean_path(&l.source_path)
                    == linkd_core::clean_path(&entry.source_path)
                    && linkd_core::clean_path(&l.consumer_root)
                        == linkd_core::clean_path(&entry.consumer_root))
                    || linkd_core::clean_path(&l.sync_target)
                        == linkd_core::clean_path(&entry.sync_target)
            }) {
                // Update existing link in-place instead of creating duplicate
                *existing = entry.clone();
                return Ok(entry);
            }
            reg.links.push(entry.clone());
            Ok(entry)
        })
    }

    pub fn remove_link(&self, package_name: &str) -> LinkdResult<Option<linkd_core::LinkEntry>> {
        self.with_mut(|reg| Ok(Registry::remove_by_package(&mut reg.links, package_name)))
    }

    pub fn remove_links_matching(
        &self,
        package_name: &str,
        consumer_root: Option<&Path>,
    ) -> LinkdResult<Vec<linkd_core::LinkEntry>> {
        self.with_mut(|reg| {
            let mut removed = Vec::new();
            reg.links.retain(|l| {
                let matches_pkg = l.package_name == package_name;
                let matches_consumer = consumer_root.map(|c| l.consumer_root == c).unwrap_or(true);
                if matches_pkg && matches_consumer {
                    removed.push(l.clone());
                    false
                } else {
                    true
                }
            });
            Ok(removed)
        })
    }

    pub fn update_link(
        &self,
        id: Uuid,
        update: impl FnOnce(&mut linkd_core::LinkEntry),
    ) -> LinkdResult<()> {
        self.with_mut(|reg| {
            let entry = Registry::find_by_id_mut(&mut reg.links, id)
                .ok_or_else(|| LinkdError::PackageNotFound(id.to_string()))?;
            update(entry);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NewLinkParams;
    use linkd_core::{IsolationMode, SyncStrategy};
    use tempfile::TempDir;

    #[test]
    fn roundtrip_registry() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("registry.json");
        let store = RegistryStore::new(path);

        let entry = Registry::new_link(NewLinkParams {
            package_name: "@test/lib".into(),
            source_path: tmp.path().join("src"),
            consumer_root: tmp.path().join("app"),
            sync_target: tmp.path().join("app/node_modules/@test/lib"),
            ecosystem: linkd_core::Ecosystem::Npm,
            link_mode: linkd_core::LinkMode::PackageManager,
            custom_target: None,
            detected_pm: None,
            strategy: SyncStrategy::Copy,
            isolation_mode: IsolationMode::ProjectLocal,
        });

        store.add_link(entry.clone()).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.links.len(), 1);
        assert_eq!(loaded.links[0].package_name, "@test/lib");
    }
}
