use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use linkd_core::{pack_cache_dir, LinkdError, LinkdResult};
use sha2::{Digest, Sha256};

pub struct PackCache;

impl PackCache {
    pub fn cache_key(source: &Path) -> LinkdResult<String> {
        let pkg_json = source.join("package.json");
        let mtime = fs::metadata(&pkg_json)
            .map_err(|e| LinkdError::io(&pkg_json, e))?
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let mtime_secs = mtime
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut hasher = Sha256::new();
        hasher.update(source.to_string_lossy().as_bytes());
        hasher.update(mtime_secs.to_le_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn cache_path(source: &Path) -> LinkdResult<PathBuf> {
        let key = Self::cache_key(source)?;
        Ok(pack_cache_dir().join(format!("{key}.json")))
    }

    pub fn read(source: &Path) -> LinkdResult<Option<Vec<PathBuf>>> {
        let path = Self::cache_path(source)?;
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(&path).map_err(|e| LinkdError::io(&path, e))?;
        let files: Vec<PathBuf> =
            serde_json::from_str(&data).map_err(|e| LinkdError::NpmPackFailed(e.to_string()))?;
        Ok(Some(files))
    }

    pub fn write(source: &Path, files: &[PathBuf]) -> LinkdResult<()> {
        fs::create_dir_all(pack_cache_dir()).map_err(|e| LinkdError::io(pack_cache_dir(), e))?;
        let path = Self::cache_path(source)?;
        let json = serde_json::to_string_pretty(files)
            .map_err(|e| LinkdError::NpmPackFailed(e.to_string()))?;
        fs::write(&path, json).map_err(|e| LinkdError::io(&path, e))?;
        Ok(())
    }

    pub fn invalidate(source: &Path) -> io::Result<()> {
        if let Ok(path) = Self::cache_path(source) {
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }
}
