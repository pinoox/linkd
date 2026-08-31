mod auth;
mod client;
pub mod events;
mod protocol;
mod server;

pub use auth::{ensure_auth_token, load_auth_token, verify_auth_token};
pub use client::IpcClient;
pub use events::DaemonEvent;
pub use protocol::{IpcRequest, IpcResponse, LinkStatusSnapshot};
pub use server::{IpcServer, ReconcileHook, ShutdownHook};
