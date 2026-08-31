mod auth;
mod client;
mod protocol;
mod server;

pub use auth::{ensure_auth_token, load_auth_token, verify_auth_token};
pub use client::IpcClient;
pub use protocol::{IpcRequest, IpcResponse, LinkStatusSnapshot};
pub use server::{IpcServer, ReconcileHook, ShutdownHook};
