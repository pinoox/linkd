use std::path::PathBuf;

use crate::error::{LinkdError, LinkdResult};
use crate::paths::{config_path, daemon_pid_path, ensure_home, linkd_home};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkdConfig {
    #[serde(default = "default_auto_start_daemon")]
    pub auto_start_daemon: bool,
}

fn default_auto_start_daemon() -> bool {
    true
}

impl Default for LinkdConfig {
    fn default() -> Self {
        Self {
            auto_start_daemon: default_auto_start_daemon(),
        }
    }
}

impl LinkdConfig {
    pub fn load() -> LinkdResult<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path).map_err(|e| LinkdError::io(&path, e))?;
        serde_json::from_str(&data).map_err(|e| LinkdError::Other(e.to_string()))
    }

    pub fn save(&self) -> LinkdResult<()> {
        ensure_home().map_err(|e| LinkdError::io(linkd_home(), e))?;
        let path = config_path();
        let json =
            serde_json::to_string_pretty(self).map_err(|e| LinkdError::Other(e.to_string()))?;
        std::fs::write(&path, json).map_err(|e| LinkdError::io(&path, e))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonPidFile {
    pub pid: u32,
    pub started_at: String,
    pub version: String,
}

impl DaemonPidFile {
    pub fn path() -> PathBuf {
        daemon_pid_path()
    }

    pub fn load() -> LinkdResult<Option<Self>> {
        let path = Self::path();
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&path).map_err(|e| LinkdError::io(&path, e))?;
        Ok(Some(
            serde_json::from_str(&data).map_err(|e| LinkdError::Other(e.to_string()))?,
        ))
    }

    pub fn save(&self) -> LinkdResult<()> {
        ensure_home().map_err(|e| LinkdError::io(linkd_home(), e))?;
        let path = Self::path();
        let json =
            serde_json::to_string_pretty(self).map_err(|e| LinkdError::Other(e.to_string()))?;
        std::fs::write(&path, json).map_err(|e| LinkdError::io(&path, e))?;
        Ok(())
    }

    pub fn remove() -> LinkdResult<()> {
        let path = Self::path();
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| LinkdError::io(&path, e))?;
        }
        Ok(())
    }
}
