use std::path::{Path, PathBuf};

use linkd_core::{IsolationMode, LinkdResult, ResolvedSyncTarget};

pub fn resolve_python_target(
    consumer_root: &Path,
    package_name: &str,
) -> LinkdResult<ResolvedSyncTarget> {
    let normalized_name = package_name.replace('-', "_");

    let venv_candidates = [
        consumer_root.join(".venv"),
        consumer_root.join("venv"),
        consumer_root.join("env"),
    ];

    let site_packages_dir = find_site_packages(&venv_candidates)
        .unwrap_or_else(|| default_site_packages(consumer_root));

    let target = site_packages_dir.join(&normalized_name);

    Ok(ResolvedSyncTarget {
        logical_target: target.clone(),
        sync_target: target,
        isolation_mode: IsolationMode::ProjectLocal,
        forbidden_roots: vec![],
    })
}

fn find_site_packages(venv_dirs: &[PathBuf]) -> Option<PathBuf> {
    for venv in venv_dirs {
        if !venv.exists() {
            continue;
        }

        // Windows standard: .venv/Lib/site-packages
        let win_site = venv.join("Lib").join("site-packages");
        if win_site.exists() {
            return Some(win_site);
        }

        let win_site_lower = venv.join("lib").join("site-packages");
        if win_site_lower.exists() {
            return Some(win_site_lower);
        }

        // Unix standard: .venv/lib/pythonX.Y/site-packages
        let lib_dir = venv.join("lib");
        if lib_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&lib_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir()
                        && path
                            .file_name()
                            .map(|n| n.to_string_lossy().starts_with("python"))
                            .unwrap_or(false)
                    {
                        let candidate = path.join("site-packages");
                        if candidate.exists() {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
    }
    None
}

fn default_site_packages(consumer_root: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        consumer_root
            .join(".venv")
            .join("Lib")
            .join("site-packages")
    }
    #[cfg(not(windows))]
    {
        consumer_root
            .join(".venv")
            .join("lib")
            .join("python3.11")
            .join("site-packages")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolves_site_packages() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let site = consumer.join(".venv").join("Lib").join("site-packages");
        std::fs::create_dir_all(&site).unwrap();

        let resolved = resolve_python_target(&consumer, "my-package").unwrap();
        assert_eq!(resolved.sync_target, site.join("my_package"));
    }
}
