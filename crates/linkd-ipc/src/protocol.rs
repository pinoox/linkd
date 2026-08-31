use linkd_core::LinkEntry;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum IpcRequest {
    Ping { auth_token: String },
    ListLinks { auth_token: String },
    AddLink {
        auth_token: String,
        entry: LinkEntry,
    },
    RemoveLink {
        auth_token: String,
        package_name: String,
    },
    GetStatus { auth_token: String },
    TriggerReconcile {
        auth_token: String,
        link_id: Option<Uuid>,
    },
    Shutdown { auth_token: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkStatusSnapshot {
    pub links: Vec<LinkEntry>,
    pub daemon_running: bool,
    pub pm_install_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IpcResponse {
    Ok {
        #[serde(default)]
        links: Vec<LinkEntry>,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        snapshot: Option<LinkStatusSnapshot>,
    },
    Error { message: String },
}

impl IpcResponse {
    pub fn err(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    pub fn ok_message(message: impl Into<String>) -> Self {
        Self::Ok {
            links: vec![],
            message: Some(message.into()),
            snapshot: None,
        }
    }
}

pub fn encode_line(value: &impl Serialize) -> Result<String, serde_json::Error> {
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    Ok(line)
}

pub fn decode_line<T: for<'de> Deserialize<'de>>(line: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(line.trim())
}
