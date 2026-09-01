mod list_files;
mod target_resolve;

pub use list_files::list_files;
pub use target_resolve::{detect_swift, parse_package_name, resolve_swift_target};

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let mut markers = vec![
        consumer_root.join("Package.resolved"),
        consumer_root.join(".build").join("workspace-state.json"),
    ];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("Package.resolved"));
    }
    markers
}

pub fn post_sync_hint(_source: &Path, _consumer: &Path) -> Option<String> {
    Some("Swift package synced into .build/checkouts. Run 'swift build' to compile.".into())
}
