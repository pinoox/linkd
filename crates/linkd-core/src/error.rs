use thiserror::Error;

use crate::types::SyncStrategy;

#[derive(Debug, Error)]
pub enum LinkdError {
    #[error("IO error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("registry error: {0}")]
    Registry(String),

    #[error("package not found: {0}")]
    PackageNotFound(String),

    #[error("daemon not running")]
    DaemonNotRunning,

    #[error("invalid IPC auth token")]
    InvalidAuthToken,

    #[error("write blocked: {0}")]
    WriteBlocked(String),

    #[error("pnpm global store write forbidden: {0}")]
    PnpmGlobalStoreForbidden(String),

    #[error("npm pack failed: {0}")]
    NpmPackFailed(String),

    #[error("unsupported package manager layout: {0}")]
    UnsupportedLayout(String),

    #[error("{0}")]
    Other(String),
}

pub type LinkdResult<T> = Result<T, LinkdError>;

impl LinkdError {
    pub fn io(path: impl AsRef<std::path::Path>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.as_ref().display().to_string(),
            source,
        }
    }
}

/// User-facing error messages (English for CLI; structured for tooling).
#[derive(Debug, Clone)]
pub struct HumanError {
    pub title: String,
    pub detail: Vec<String>,
    pub hint: Option<String>,
}

impl HumanError {
    pub fn from_error(err: &LinkdError) -> Self {
        match err {
            LinkdError::PnpmGlobalStoreForbidden(path) => HumanError {
                title: "Cannot write directly to pnpm global store".into(),
                detail: vec![
                    "This could affect other projects on your machine.".into(),
                    format!("Blocked path: {path}"),
                ],
                hint: Some("linkd created an isolated shadow copy instead. Run: linkd doctor --explain pnpm-store".into()),
            },
            LinkdError::WriteBlocked(msg) => HumanError {
                title: "Write operation blocked by safety guard".into(),
                detail: vec![msg.clone()],
                hint: Some("Run: linkd doctor".into()),
            },
            LinkdError::DaemonNotRunning => HumanError {
                title: "linkd daemon is not running".into(),
                detail: vec![
                    "Start background daemon: linkd start".into(),
                    "Or foreground with UI: linkd watch".into(),
                ],
                hint: None,
            },
            LinkdError::NpmPackFailed(msg) => HumanError {
                title: "Failed to list pack files via npm".into(),
                detail: vec![msg.clone()],
                hint: Some("Ensure npm is installed and package.json is valid.".into()),
            },
            _ => HumanError {
                title: err.to_string(),
                detail: vec![],
                hint: None,
            },
        }
    }

    pub fn hardlink_warning() -> Self {
        HumanError {
            title: "Hardlink mode enabled".into(),
            detail: vec![
                "Edits under node_modules may silently modify your source files.".into(),
                "Use copy/reflink (default) unless you understand the risk.".into(),
            ],
            hint: None,
        }
    }

    pub fn strategy_warning(strategy: SyncStrategy) -> Option<Self> {
        match strategy {
            SyncStrategy::Hardlink => Some(Self::hardlink_warning()),
            _ => None,
        }
    }

    pub fn display(&self) -> String {
        let mut out = format!("❌ {}\n", self.title);
        for line in &self.detail {
            out.push_str(&format!("   {line}\n"));
        }
        if let Some(hint) = &self.hint {
            out.push_str(&format!("   → {hint}\n"));
        }
        out
    }
}
