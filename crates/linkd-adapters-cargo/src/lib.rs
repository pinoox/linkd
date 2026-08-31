mod detect;
mod list_files;
mod target_resolve;

pub use detect::{detect_cargo, parse_crate_name};
pub use list_files::list_files;
pub use target_resolve::resolve_cargo_target;

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let mut markers = vec![
        consumer_root.join("Cargo.lock"),
        consumer_root.join(".cargo").join("config.toml"),
        consumer_root.join(".cargo").join("config"),
    ];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("Cargo.toml"));
    }
    markers
}

pub fn post_sync_hint(source: &Path, _consumer: &Path) -> Option<String> {
    if source.join("Cargo.toml").exists() {
        Some("Rust crate files synced to vendor. Ensure `.cargo/config.toml` has `[source.crates-io] replace-with = 'vendored-sources'` if vendoring.".into())
    } else {
        None
    }
}
