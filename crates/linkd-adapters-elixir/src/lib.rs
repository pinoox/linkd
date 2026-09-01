mod list_files;
mod target_resolve;

pub use list_files::list_files;
pub use target_resolve::{detect_elixir, parse_package_name, resolve_elixir_target};

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let mut markers = vec![consumer_root.join("mix.lock"), consumer_root.join("_build")];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("mix.lock"));
    }
    markers
}

pub fn post_sync_hint(_source: &Path, _consumer: &Path) -> Option<String> {
    Some("Elixir dependency synced into deps/ folder. Run 'mix compile' to compile.".into())
}
