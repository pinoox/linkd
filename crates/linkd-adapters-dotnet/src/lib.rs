mod list_files;
mod target_resolve;

pub use list_files::list_files;
pub use target_resolve::{detect_dotnet, parse_package_name, resolve_dotnet_target};

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let mut markers = vec![
        consumer_root.join("obj").join("project.assets.json"),
        consumer_root.join("packages.lock.json"),
        consumer_root.join("Directory.Packages.props"),
    ];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("obj").join("project.assets.json"));
    }
    markers
}

pub fn post_sync_hint(_source: &Path, _consumer: &Path) -> Option<String> {
    Some(".NET package synced into packages/ folder. Run 'dotnet build' to compile.".into())
}
