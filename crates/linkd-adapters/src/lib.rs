mod custom;
mod dispatch;
mod validate;

pub use custom::{
    CargoAdapter, ComposerAdapter, CustomAdapter, DartAdapter, GoAdapter, JvmAdapter, NpmAdapter,
    PythonAdapter,
};
pub use dispatch::{
    adapter_for, build_allowlist_for_link, completion_markers_for_link, detect_ecosystem,
    ensure_isolation, list_files_for_link, post_sync_hint_for_link, resolve_for_reconcile,
    resolve_link, ResolvedLink,
};
pub use validate::validate_link_paths;

use std::path::{Path, PathBuf};

use linkd_core::{Ecosystem, LinkdResult, ResolvedSyncTarget};

/// Per-ecosystem sync operations.
pub trait EcosystemAdapter: Send + Sync {
    fn ecosystem(&self) -> Ecosystem;

    fn detect(&self, source: &Path, consumer: &Path) -> bool;

    fn package_name(&self, source: &Path) -> LinkdResult<String>;

    fn resolve_target(
        &self,
        consumer: &Path,
        package_name: &str,
        custom_target: Option<&Path>,
    ) -> LinkdResult<ResolvedSyncTarget>;

    fn completion_markers(&self, consumer: &Path) -> Vec<PathBuf>;

    fn list_files(&self, source: &Path) -> LinkdResult<Vec<PathBuf>>;

    fn write_guard_roots(&self, consumer: &Path) -> Vec<PathBuf>;

    fn post_sync_hint(&self, _source: &Path, _consumer: &Path) -> Option<String> {
        None
    }
}

pub fn ensure_shadow_isolation(
    consumer_root: &Path,
    package_name: &str,
    resolved: &ResolvedSyncTarget,
) -> LinkdResult<()> {
    linkd_adapters_npm::ensure_shadow_isolation(consumer_root, package_name, resolved)
}

pub fn assert_never_writes_global_store(sync_target: &Path) -> LinkdResult<()> {
    linkd_adapters_npm::PnpmStoreDetector::assert_never_writes_global_store(sync_target)
}

pub fn build_allowlist(
    consumer_root: &Path,
    forbidden: Vec<PathBuf>,
) -> linkd_sync::WriteAllowlist {
    linkd_adapters_npm::PnpmStoreDetector::build_allowlist(consumer_root, forbidden)
}
