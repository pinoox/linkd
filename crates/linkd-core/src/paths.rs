use std::path::{Path, PathBuf};

use directories::ProjectDirs;

const APP_QUALIFIER: &str = "dev";
const APP_ORG: &str = "linkd";
const APP_NAME: &str = "linkd";

/// Resolve `~/.linkd` (or XDG data dir / `%LOCALAPPDATA%` on Windows).
pub fn linkd_home() -> PathBuf {
    if let Ok(custom) = std::env::var("LINKD_HOME") {
        return PathBuf::from(custom);
    }

    ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME)
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| dirs_fallback_home().join(".linkd"))
}

fn dirs_fallback_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn registry_path() -> PathBuf {
    linkd_home().join("registry.json")
}

pub fn auth_token_path() -> PathBuf {
    linkd_home().join("auth_token")
}

pub fn daemon_socket_path() -> PathBuf {
    linkd_home().join("daemon.sock")
}

#[cfg(windows)]
pub fn daemon_pipe_name() -> String {
    r"\\.\pipe\linkd-daemon".to_string()
}

pub fn log_path() -> PathBuf {
    linkd_home().join("linkd.log")
}

pub fn tmp_dir() -> PathBuf {
    linkd_home().join("tmp")
}

pub fn pack_cache_dir() -> PathBuf {
    linkd_home().join("pack-cache")
}

pub fn config_path() -> PathBuf {
    linkd_home().join("config.json")
}

pub fn daemon_pid_path() -> PathBuf {
    linkd_home().join("daemon.pid")
}

pub fn ensure_home() -> std::io::Result<()> {
    let home = linkd_home();
    std::fs::create_dir_all(&home)?;
    std::fs::create_dir_all(tmp_dir())?;
    std::fs::create_dir_all(pack_cache_dir())?;
    Ok(())
}

pub fn is_ci() -> bool {
    std::env::var("CI")
        .map(|v| !v.is_empty() && v != "0" && v.to_lowercase() != "false")
        .unwrap_or(false)
}

pub fn pinned_packages_path() -> PathBuf {
    linkd_home().join("packages.json")
}

pub fn clean_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
            PathBuf::from(format!(r"\\{}", stripped))
        } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
            PathBuf::from(stripped)
        } else {
            path.to_path_buf()
        }
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

pub fn display_path(path: &Path) -> String {
    clean_path(path).to_string_lossy().to_string()
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(path)
    } else {
        path.to_path_buf()
    };

    let canonical = abs.canonicalize().unwrap_or_else(|_| {
        let mut curr = abs.clone();
        let mut rel_components = Vec::new();
        while !curr.exists() {
            if let Some(name) = curr.file_name() {
                rel_components.push(name.to_os_string());
                if let Some(parent) = curr.parent() {
                    curr = parent.to_path_buf();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if let Ok(mut base) = curr.canonicalize() {
            for comp in rel_components.into_iter().rev() {
                base.push(comp);
            }
            base
        } else {
            abs
        }
    });

    clean_path(&canonical)
}

#[cfg(unix)]
pub fn set_owner_only_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(windows)]
pub fn set_owner_only_permissions(_path: &Path) -> std::io::Result<()> {
    // Named pipe DACL is configured at creation time in linkd-ipc.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn linkd_home_respects_env() {
        let tmp = TempDir::new().unwrap();
        std::env::set_var("LINKD_HOME", tmp.path());
        assert_eq!(linkd_home(), tmp.path());
        std::env::remove_var("LINKD_HOME");
    }
}
