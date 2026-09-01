mod list_files;
mod target_resolve;

pub use list_files::list_files;
pub use target_resolve::{detect_ruby, parse_package_name, resolve_ruby_target};

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let mut markers = vec![
        consumer_root.join("Gemfile.lock"),
        consumer_root.join(".bundle").join("config"),
    ];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("Gemfile.lock"));
    }
    markers
}

pub fn post_sync_hint(_source: &Path, _consumer: &Path) -> Option<String> {
    Some("Ruby gem synced into vendor/bundle/gems. Available to bundler.".into())
}
