use std::path::{Path, PathBuf};
use std::process::Command;

use linkd_core::{LinkdError, LinkdResult};

use crate::cache::PackCache;

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

        #[cfg(windows)]
        let npm_bin = "npm.cmd";
        #[cfg(not(windows))]
        let npm_bin = "npm";

        let output = Command::new(npm_bin)
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
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let mut paths = Vec::new();
        extract_paths_from_value(&value, &mut paths);
        if !paths.is_empty() {
            paths.sort();
            paths.dedup();
            return Ok(paths);
        }
    }

    let mut files = Vec::new();
    for line in trimmed.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(l) {
            extract_paths_from_value(&value, &mut files);
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn extract_paths_from_value(val: &serde_json::Value, out: &mut Vec<PathBuf>) {
    match val {
        serde_json::Value::Array(arr) => {
            for item in arr {
                extract_paths_from_value(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(files) = map.get("files") {
                extract_paths_from_value(files, out);
            } else if let Some(path_val) = map.get("path").and_then(|p| p.as_str()) {
                out.push(PathBuf::from(path_val));
            }
        }
        _ => {}
    }
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

    // Walk the entire source tree, excluding well-known noise dirs
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Ok(rel) = path.strip_prefix(source) {
            let skip = rel.components().any(|c| {
                matches!(
                    c.as_os_str().to_string_lossy().as_ref(),
                    "node_modules" | ".git" | "target" | ".linkd-shadow"
                )
            });
            if !skip {
                files.push(rel.to_path_buf());
            }
        }
    }
    files.sort();
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

    #[test]
    fn parse_json_npm_array_format() {
        let input = r#"[
  {
    "id": "@test/npm-lib@1.0.0",
    "name": "@test/npm-lib",
    "version": "1.0.0",
    "files": [
      {
        "path": "index.js",
        "size": 51
      },
      {
        "path": "package.json",
        "size": 54
      }
    ]
  }
]"#;
        let files = parse_npm_pack_json(input).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&PathBuf::from("index.js")));
        assert!(files.contains(&PathBuf::from("package.json")));
    }
}
