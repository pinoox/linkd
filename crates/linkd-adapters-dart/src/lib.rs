mod list_files;
mod target_resolve;

pub use list_files::list_files;
pub use target_resolve::{detect_dart, parse_package_name, resolve_dart_target};

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let mut markers = vec![
        consumer_root.join("pubspec.lock"),
        consumer_root.join(".dart_tool").join("package_config.json"),
        consumer_root.join(".packages"),
    ];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("pubspec.lock"));
    }
    markers
}

pub fn post_sync_hint(_source: &Path, _consumer: &Path) -> Option<String> {
    Some("Flutter/Dart package synced into .dart_tool/packages".into())
}
