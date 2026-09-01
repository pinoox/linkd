use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use linkd_core::{normalize_path, pinned_packages_path, Ecosystem, LinkdError, LinkdResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedPackage {
    pub name: String,
    pub path: PathBuf,
    pub ecosystem: Ecosystem,
    pub pinned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PinnedFile {
    #[serde(default)]
    pub packages: BTreeMap<String, PinnedPackage>,
}

pub struct PinnedStore {
    path: PathBuf,
}

impl Default for PinnedStore {
    fn default() -> Self {
        Self::new(pinned_packages_path())
    }
}

impl PinnedStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> LinkdResult<PinnedFile> {
        if !self.path.exists() {
            return Ok(PinnedFile::default());
        }
        let content =
            std::fs::read_to_string(&self.path).map_err(|e| LinkdError::io(&self.path, e))?;
        if content.trim().is_empty() {
            return Ok(PinnedFile::default());
        }
        serde_json::from_str(&content).map_err(|e| LinkdError::Registry(e.to_string()))
    }

    pub fn save(&self, file: &PinnedFile) -> LinkdResult<()> {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json =
            serde_json::to_string_pretty(file).map_err(|e| LinkdError::Registry(e.to_string()))?;
        std::fs::write(&self.path, json).map_err(|e| LinkdError::io(&self.path, e))
    }

    pub fn pin(
        &self,
        name: String,
        path: PathBuf,
        ecosystem: Ecosystem,
    ) -> LinkdResult<PinnedPackage> {
        let mut file = self.load()?;
        let pkg = PinnedPackage {
            name: name.clone(),
            path: normalize_path(&path),
            ecosystem,
            pinned_at: Utc::now(),
        };
        file.packages.insert(name, pkg.clone());
        self.save(&file)?;
        Ok(pkg)
    }

    pub fn unpin(&self, name: &str) -> LinkdResult<Option<PinnedPackage>> {
        let mut file = self.load()?;
        let removed = file.packages.remove(name);
        if removed.is_some() {
            self.save(&file)?;
        }
        Ok(removed)
    }

    pub fn get(&self, name: &str) -> LinkdResult<Option<PinnedPackage>> {
        let file = self.load()?;
        Ok(file.packages.get(name).cloned())
    }

    pub fn list(&self) -> LinkdResult<Vec<PinnedPackage>> {
        let file = self.load()?;
        Ok(file.packages.into_values().collect())
    }
}
