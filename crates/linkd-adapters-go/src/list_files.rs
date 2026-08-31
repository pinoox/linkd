use std::path::{Path, PathBuf};

use linkd_core::LinkdResult;
use walkdir::WalkDir;

pub fn list_files(source_root: &Path) -> LinkdResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(source_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let rel = path
                .strip_prefix(source_root)
                .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;

            if is_ignored(rel) {
                continue;
            }
            files.push(rel.to_path_buf());
        }
    }

    files.sort();
    Ok(files)
}

fn is_ignored(rel: &Path) -> bool {
    rel.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        name == "vendor" || name == ".git" || name == "bin" || name == ".idea" || name == ".vscode"
    })
}
