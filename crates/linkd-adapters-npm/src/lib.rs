mod detect;
mod pnpm_store;
mod target_resolve;

pub use detect::{completion_markers, detect_package_manager, is_yarn_pnp, PackageManager};
pub use pnpm_store::{ensure_shadow_isolation, PnpmStoreDetector};
pub use target_resolve::{parse_package_name, resolve_node_modules_target, shadow_target_path};

use std::path::{Path, PathBuf};

pub fn completion_markers_for_link(consumer_root: &Path) -> Vec<PathBuf> {
    let pm = detect_package_manager(consumer_root);
    completion_markers(consumer_root, pm)
}
