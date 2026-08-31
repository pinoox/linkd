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
        .unwrap_or_else(|| {
            dirs_fallback_home().join(".linkd")
        })
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

pub fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
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
