use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub fn list_files(source: &Path) -> linkd_core::LinkdResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let rel = path
                .strip_prefix(source)
                .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
            if should_exclude(rel) {
                continue;
            }
            files.push(rel.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn should_exclude(rel: &Path) -> bool {
    rel.components().any(|c| {
        matches!(
            c.as_os_str().to_string_lossy().as_ref(),
            ".git" | "node_modules" | "vendor" | "target"
        )
    })
}
