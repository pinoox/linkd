use std::path::{Path, PathBuf};

use linkd_core::{Ecosystem, IsolationMode, LinkdResult, ResolvedSyncTarget};
use linkd_pack::{list_pack_files_cached, list_pack_files_fallback};
use walkdir::WalkDir;

use crate::EcosystemAdapter;

pub struct CustomAdapter;

impl EcosystemAdapter for CustomAdapter {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Custom
    }

    fn detect(&self, _source: &Path, _consumer: &Path) -> bool {
        false
    }

    fn package_name(&self, source: &Path) -> LinkdResult<String> {
        source
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from)
            .ok_or_else(|| linkd_core::LinkdError::Other("invalid source path".into()))
    }

    fn resolve_target(
        &self,
        _consumer: &Path,
        _package_name: &str,
        custom_target: Option<&Path>,
    ) -> LinkdResult<ResolvedSyncTarget> {
        let target = custom_target
            .ok_or_else(|| linkd_core::LinkdError::Other("custom target required".into()))?
            .to_path_buf();

        Ok(ResolvedSyncTarget {
            logical_target: target.clone(),
            sync_target: target,
            isolation_mode: IsolationMode::ProjectLocal,
            forbidden_roots: vec![],
        })
    }

    fn completion_markers(&self, _consumer: &Path) -> Vec<PathBuf> {
        vec![]
    }

    fn list_files(&self, source: &Path) -> LinkdResult<Vec<PathBuf>> {
        list_files_walkdir(source)
    }

    fn write_guard_roots(&self, consumer: &Path) -> Vec<PathBuf> {
        vec![consumer.to_path_buf()]
    }
}

pub fn list_files_walkdir(source: &Path) -> LinkdResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if should_exclude(path) {
                continue;
            }
            let rel = path
                .strip_prefix(source)
                .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
            files.push(rel.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn should_exclude(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c.as_os_str().to_string_lossy().as_ref(),
            ".git" | "node_modules" | "vendor" | "target" | ".linkd-shadow"
        )
    })
}

pub struct NpmAdapter;

impl EcosystemAdapter for NpmAdapter {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Npm
    }

    fn detect(&self, source: &Path, _consumer: &Path) -> bool {
        source.join("package.json").is_file()
    }

    fn package_name(&self, source: &Path) -> LinkdResult<String> {
        linkd_adapters_npm::parse_package_name(source)
    }

    fn resolve_target(
        &self,
        consumer: &Path,
        package_name: &str,
        _custom_target: Option<&Path>,
    ) -> LinkdResult<ResolvedSyncTarget> {
        linkd_adapters_npm::PnpmStoreDetector::resolve(consumer, package_name)
    }

    fn completion_markers(&self, consumer: &Path) -> Vec<PathBuf> {
        linkd_adapters_npm::completion_markers_for_link(consumer)
    }

    fn list_files(&self, source: &Path) -> LinkdResult<Vec<PathBuf>> {
        match list_pack_files_cached(source) {
            Ok(f) => Ok(f),
            Err(_) => list_pack_files_fallback(source),
        }
    }

    fn write_guard_roots(&self, _consumer: &Path) -> Vec<PathBuf> {
        vec![]
    }
}

pub struct ComposerAdapter;

impl EcosystemAdapter for ComposerAdapter {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Composer
    }

    fn detect(&self, source: &Path, _consumer: &Path) -> bool {
        source.join("composer.json").is_file()
    }

    fn package_name(&self, source: &Path) -> LinkdResult<String> {
        linkd_adapters_composer::parse_package_name(source)
    }

    fn resolve_target(
        &self,
        consumer: &Path,
        package_name: &str,
        _custom_target: Option<&Path>,
    ) -> LinkdResult<ResolvedSyncTarget> {
        linkd_adapters_composer::resolve_vendor_target(consumer, package_name)
    }

    fn completion_markers(&self, consumer: &Path) -> Vec<PathBuf> {
        linkd_adapters_composer::completion_markers(consumer)
    }

    fn list_files(&self, source: &Path) -> LinkdResult<Vec<PathBuf>> {
        linkd_adapters_composer::list_files(source)
    }

    fn write_guard_roots(&self, consumer: &Path) -> Vec<PathBuf> {
        vec![consumer.join("vendor")]
    }

    fn post_sync_hint(&self, source: &Path, consumer: &Path) -> Option<String> {
        linkd_adapters_composer::autoload_hint(source, consumer)
    }
}
