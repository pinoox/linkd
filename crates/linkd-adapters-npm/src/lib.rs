mod detect;
mod pnpm_store;
mod target_resolve;

pub use detect::{completion_markers, detect_package_manager, is_yarn_pnp, PackageManager};
pub use pnpm_store::{ensure_shadow_isolation, PnpmStoreDetector, ResolvedSyncTarget};
pub use target_resolve::{parse_package_name, resolve_node_modules_target, shadow_target_path};
