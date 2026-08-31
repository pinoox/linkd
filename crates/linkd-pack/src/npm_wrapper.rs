use std::path::{Path, PathBuf};
use std::process::Command;

use linkd_core::{LinkdError, LinkdResult};

use crate::cache::PackCache;

#[derive(Debug, Clone, serde::Deserialize)]
struct NpmPackEntry {
    path: String,
}

pub struct NpmPackList;

impl NpmPackList {
    pub fn from_source(source: &Path) -> LinkdResult<Vec<PathBuf>> {
        let pkg_json = source.join("package.json");
        if !pkg_json.exists() {
            return Err(LinkdError::NpmPackFailed(format!(
                "no package.json in {}",
                source.display()
            )));
        }

        let output = Command::new("npm")
            .arg("pack")
            .arg("--dry-run")
            .arg("--json")
            .current_dir(source)
            .output()
            .map_err(|e| LinkdError::NpmPackFailed(format!("failed to run npm: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LinkdError::NpmPackFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_npm_pack_json(&stdout)
    }
}

fn parse_npm_pack_json(stdout: &str) -> LinkdResult<Vec<PathBuf>> {
    // npm may emit one JSON object per line or a single array
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') {
        let entries: Vec<NpmPackEntry> =
            serde_json::from_str(trimmed).map_err(|e| LinkdError::NpmPackFailed(e.to_string()))?;
        return Ok(entries.into_iter().map(|e| PathBuf::from(e.path)).collect());
    }

    let mut files = Vec::new();
    for line in trimmed.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: NpmPackEntry =
            serde_json::from_str(line).map_err(|e| LinkdError::NpmPackFailed(e.to_string()))?;
        files.push(PathBuf::from(entry.path));
    }
    Ok(files)
}

pub fn list_pack_files(source: &Path) -> LinkdResult<Vec<PathBuf>> {
    NpmPackList::from_source(source)
}

pub fn list_pack_files_cached(source: &Path) -> LinkdResult<Vec<PathBuf>> {
    if let Some(cached) = PackCache::read(source)? {
        return Ok(cached);
    }

    let files = list_pack_files(source)?;
    PackCache::write(source, &files)?;
    Ok(files)
}

/// Fallback when npm is unavailable (tests / minimal fixtures).
pub fn list_pack_files_fallback(source: &Path) -> LinkdResult<Vec<PathBuf>> {
    let pkg = source.join("package.json");
    if !pkg.exists() {
        return Err(LinkdError::NpmPackFailed("missing package.json".into()));
    }

    let mut files = vec![PathBuf::from("package.json")];
    let index = source.join("index.js");
    if index.exists() {
        files.push(PathBuf::from("index.js"));
    }
    let src_index = source.join("src").join("index.js");
    if src_index.exists() {
        files.push(PathBuf::from("src/index.js"));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_lines() {
        let input = r#"{"path":"package.json"}
{"path":"index.js"}"#;
        let files = parse_npm_pack_json(input).unwrap();
        assert_eq!(files.len(), 2);
    }
}
