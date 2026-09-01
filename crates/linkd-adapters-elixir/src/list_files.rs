use std::path::{Path, PathBuf};

use linkd_core::LinkdResult;
use walkdir::WalkDir;

pub fn list_files(source: &Path) -> LinkdResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if should_exclude(path) {
                continue;
            }
            let rel = path
                .strip_prefix(source)
                .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
            files.push(rel.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn should_exclude(path: &Path) -> bool {
    path.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        matches!(
            name.as_ref(),
            ".git" | "_build" | "deps" | ".elixir_ls" | ".linkd-shadow"
        )
    })
}
