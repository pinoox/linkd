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

        // 1. Exact match
        if let Some(pkg) = file.packages.get(name) {
            return Ok(Some(pkg.clone()));
        }

        let name_lower = name.to_lowercase();
        let name_norm = name_lower.replace('_', "-");

        // 2. Case-insensitive / normalized key match
        for (k, pkg) in &file.packages {
            let k_lower = k.to_lowercase();
            if k_lower == name_lower || k_lower.replace('_', "-") == name_norm {
                return Ok(Some(pkg.clone()));
            }
        }

        // 3. Match against folder name (e.g. user typed folder "go-lib" or "php-lib")
        for pkg in file.packages.values() {
            if let Some(dir_name) = pkg
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
            {
                if dir_name == name_lower || dir_name.replace('_', "-") == name_norm {
                    return Ok(Some(pkg.clone()));
                }
            }
        }

        // 4. Match against package suffix/tail (e.g. "example.com/go-lib" -> "go-lib", "vendor/php-lib" -> "php-lib", "com.example:jvm-lib" -> "jvm-lib")
        for (k, pkg) in &file.packages {
            let k_lower = k.to_lowercase();
            let tail = k_lower
                .split(['/', ':', '@'])
                .next_back()
                .unwrap_or(&k_lower);
            if tail == name_lower || tail.replace('_', "-") == name_norm {
                return Ok(Some(pkg.clone()));
            }
        }

        Ok(None)
    }

    pub fn list(&self) -> LinkdResult<Vec<PinnedPackage>> {
        let file = self.load()?;
        Ok(file.packages.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_pinned_store_smart_matching() {
        let tmp = TempDir::new().unwrap();
        let store = PinnedStore::new(tmp.path().join("packages.json"));

        let go_path = tmp.path().join("go-lib");
        std::fs::create_dir_all(&go_path).unwrap();
        store
            .pin(
                "example.com/org/go-lib".into(),
                go_path.clone(),
                Ecosystem::Go,
            )
            .unwrap();

        let dart_path = tmp.path().join("dart-pkg");
        std::fs::create_dir_all(&dart_path).unwrap();
        store
            .pin("dart_pkg".into(), dart_path.clone(), Ecosystem::Dart)
            .unwrap();

        let php_path = tmp.path().join("php-lib");
        std::fs::create_dir_all(&php_path).unwrap();
        store
            .pin(
                "vendor/php-lib".into(),
                php_path.clone(),
                Ecosystem::Composer,
            )
            .unwrap();

        // Exact match
        assert!(store.get("example.com/org/go-lib").unwrap().is_some());

        // Suffix match
        assert_eq!(
            store.get("go-lib").unwrap().unwrap().name,
            "example.com/org/go-lib"
        );
        assert_eq!(
            store.get("php-lib").unwrap().unwrap().name,
            "vendor/php-lib"
        );

        // Snake_case vs kebab-case match
        assert_eq!(store.get("dart-pkg").unwrap().unwrap().name, "dart_pkg");
        assert_eq!(store.get("dart_pkg").unwrap().unwrap().name, "dart_pkg");
    }
}
