mod detect;
mod list_files;
mod target_resolve;

pub use detect::{detect_go, parse_module_name};
pub use list_files::list_files;
pub use target_resolve::resolve_go_target;

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let mut markers = vec![
        consumer_root.join("go.sum"),
        consumer_root.join("go.work.sum"),
        consumer_root.join("go.work"),
        consumer_root.join("vendor").join("modules.txt"),
    ];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("go.mod"));
    }
    markers
}

pub fn post_sync_hint(source: &Path, _consumer: &Path) -> Option<String> {
    if source.join("go.mod").exists() {
        Some("Go module files synced to vendor. Run with `go build -mod=vendor` or use `go work` for multi-module setup.".into())
    } else {
        None
    }
}
