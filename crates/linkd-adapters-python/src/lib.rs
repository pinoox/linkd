mod detect;
mod list_files;
mod target_resolve;

pub use detect::{detect_python, parse_package_name};
pub use list_files::list_files;
pub use target_resolve::resolve_python_target;

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let mut markers = vec![
        consumer_root.join("uv.lock"),
        consumer_root.join("poetry.lock"),
        consumer_root.join("Pipfile.lock"),
        consumer_root.join("requirements.txt"),
        consumer_root.join("pyproject.toml"),
        consumer_root.join(".venv").join("pyvenv.cfg"),
        consumer_root.join("venv").join("pyvenv.cfg"),
    ];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("pyproject.toml"));
    }
    markers
}

pub fn post_sync_hint(source: &Path, _consumer: &Path) -> Option<String> {
    if source.join("pyproject.toml").exists() || source.join("setup.py").exists() {
        Some("Python files synced to site-packages. Restart running python processes or servers to pick up module changes.".into())
    } else {
        None
    }
}
