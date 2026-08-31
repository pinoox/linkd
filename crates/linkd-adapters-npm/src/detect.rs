use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Unknown,
}

pub fn detect_package_manager(consumer_root: &Path) -> PackageManager {
    if consumer_root.join("pnpm-lock.yaml").exists()
        || consumer_root.join("pnpm-workspace.yaml").exists()
    {
        return PackageManager::Pnpm;
    }
    if consumer_root.join("yarn.lock").exists() {
        return PackageManager::Yarn;
    }
    if consumer_root.join("bun.lockb").exists() {
        return PackageManager::Bun;
    }
    if consumer_root.join("package-lock.json").exists()
        || consumer_root
            .join("node_modules")
            .join(".package-lock.json")
            .exists()
    {
        return PackageManager::Npm;
    }
    PackageManager::Unknown
}

pub fn is_yarn_pnp(consumer_root: &Path) -> bool {
    consumer_root.join(".pnp.cjs").exists() || consumer_root.join(".pnp.js").exists()
}

pub fn completion_markers(consumer_root: &Path, pm: PackageManager) -> Vec<PathBuf> {
    let nm = consumer_root.join("node_modules");
    match pm {
        PackageManager::Npm => vec![nm.join(".package-lock.json")],
        PackageManager::Pnpm => vec![nm.join(".modules.yaml")],
        PackageManager::Yarn => vec![nm.join(".yarn-integrity")],
        PackageManager::Bun => {
            let mut v = vec![consumer_root.join("bun.lockb")];
            let tag = nm.join(".bun-tag");
            if tag.exists() {
                v.push(tag);
            }
            v
        }
        PackageManager::Unknown => vec![nm.join(".package-lock.json"), nm.join(".modules.yaml")],
    }
}
