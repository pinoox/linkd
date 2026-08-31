use std::sync::Arc;

use linkd_core::LinkdResult;
use linkd_registry::RegistryStore;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::auth::verify_auth_token;
use crate::protocol::{decode_line, encode_line, IpcRequest, IpcResponse, LinkStatusSnapshot};

pub type ReconcileHook = Arc<dyn Fn(Option<uuid::Uuid>) + Send + Sync>;

pub struct IpcServer {
    registry: RegistryStore,
    on_reconcile: Option<ReconcileHook>,
    pm_hint: Arc<Mutex<Option<String>>>,
}

impl IpcServer {
    pub fn new(registry: RegistryStore) -> Self {
        Self {
            registry,
            on_reconcile: None,
            pm_hint: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_reconcile_hook(mut self, hook: ReconcileHook) -> Self {
        self.on_reconcile = Some(hook);
        self
    }

    pub async fn run(self) -> LinkdResult<()> {
        linkd_core::ensure_home().map_err(|e| linkd_core::LinkdError::io(linkd_core::linkd_home(), e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            use tokio::net::UnixListener;

            let path = linkd_core::daemon_socket_path();
            if path.exists() {
                let _ = std::fs::remove_file(&path);
            }

            let listener = UnixListener::bind(&path).map_err(|e| linkd_core::LinkdError::io(&path, e))?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| linkd_core::LinkdError::io(&path, e))?;

            info!("IPC listening on {}", path.display());

            loop {
                let (stream, _) = listener
                    .accept()
                    .await
                    .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
                let server = self.clone_inner();
                tokio::spawn(async move {
                    if let Err(e) = server.handle_connection(stream).await {
                        error!("ipc connection error: {e}");
                    }
                });
            }
        }

        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ServerOptions;

            loop {
                let pipe = ServerOptions::new()
                    .first_pipe_instance(true)
                    .create(linkd_core::daemon_pipe_name())
                    .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;

                let server = self.clone_inner();
                if let Err(e) = server.handle_connection_windows(pipe).await {
                    error!("ipc pipe error: {e}");
                }
            }
        }
    }

    fn clone_inner(&self) -> Self {
        Self {
            registry: RegistryStore::new(self.registry.path().to_path_buf()),
            on_reconcile: self.on_reconcile.clone(),
            pm_hint: self.pm_hint.clone(),
        }
    }

    #[cfg(unix)]
    async fn handle_connection(&self, stream: tokio::net::UnixStream) -> LinkdResult<()> {
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?
        {
            let resp = self.dispatch_line(&line).await;
            let payload = encode_line(&resp).map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
            writer
                .write_all(payload.as_bytes())
                .await
                .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
        }
        Ok(())
    }

    #[cfg(windows)]
    async fn handle_connection_windows(
        &self,
        mut pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    ) -> LinkdResult<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        pipe.connect()
            .await
            .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;

        let mut buf = vec![0u8; 65536];
        let n = pipe
            .read(&mut buf)
            .await
            .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
        let line = String::from_utf8_lossy(&buf[..n]);
        let resp = self.dispatch_line(&line).await;
        let payload = encode_line(&resp).map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
        pipe.write_all(payload.as_bytes())
            .await
            .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
        Ok(())
    }

    async fn dispatch_line(&self, line: &str) -> IpcResponse {
        let req: IpcRequest = match decode_line(line) {
            Ok(r) => r,
            Err(e) => return IpcResponse::err(e.to_string()),
        };

        let token = match &req {
            IpcRequest::Ping { auth_token, .. }
            | IpcRequest::ListLinks { auth_token, .. }
            | IpcRequest::AddLink { auth_token, .. }
            | IpcRequest::RemoveLink { auth_token, .. }
            | IpcRequest::GetStatus { auth_token, .. }
            | IpcRequest::TriggerReconcile { auth_token, .. }
            | IpcRequest::Shutdown { auth_token, .. } => auth_token.clone(),
        };

        if let Err(e) = verify_auth_token(&token) {
            return IpcResponse::err(e.to_string());
        }

        match req {
            IpcRequest::Ping { .. } => IpcResponse::ok_message("pong"),
            IpcRequest::ListLinks { .. } => match self.registry.load() {
                Ok(reg) => IpcResponse::Ok {
                    links: reg.links,
                    message: None,
                    snapshot: None,
                },
                Err(e) => IpcResponse::err(e.to_string()),
            },
            IpcRequest::AddLink { entry, .. } => match self.registry.add_link(entry) {
                Ok(_) => {
                    if let Some(hook) = &self.on_reconcile {
                        hook(None);
                    }
                    IpcResponse::ok_message("link added")
                }
                Err(e) => IpcResponse::err(e.to_string()),
            },
            IpcRequest::RemoveLink { package_name, .. } => {
                match self.registry.remove_link(&package_name) {
                    Ok(_) => IpcResponse::ok_message("link removed"),
                    Err(e) => IpcResponse::err(e.to_string()),
                }
            }
            IpcRequest::GetStatus { .. } => match self.registry.load() {
                Ok(reg) => {
                    let hint = self.pm_hint.lock().await.clone();
                    IpcResponse::Ok {
                        links: reg.links.clone(),
                        message: None,
                        snapshot: Some(LinkStatusSnapshot {
                            links: reg.links,
                            daemon_running: true,
                            pm_install_hint: hint,
                        }),
                    }
                }
                Err(e) => IpcResponse::err(e.to_string()),
            },
            IpcRequest::TriggerReconcile { link_id, .. } => {
                if let Some(hook) = &self.on_reconcile {
                    hook(link_id);
                }
                IpcResponse::ok_message("reconcile queued")
            }
            IpcRequest::Shutdown { .. } => {
                info!("shutdown requested");
                IpcResponse::ok_message("shutting down")
            }
        }
    }
}
