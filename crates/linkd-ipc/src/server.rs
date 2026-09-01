use std::sync::Arc;

use linkd_core::{LinkSyncStatus, LinkdResult};
use linkd_registry::RegistryStore;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info};

use crate::auth::verify_auth_token;
use crate::events::DaemonEvent;
use crate::protocol::{decode_line, encode_line, IpcRequest, IpcResponse, LinkStatusSnapshot};

pub type ReconcileHook = Arc<dyn Fn(Option<uuid::Uuid>) + Send + Sync>;
pub type ShutdownHook = Arc<dyn Fn() + Send + Sync>;

pub struct IpcServer {
    registry: RegistryStore,
    on_reconcile: Option<ReconcileHook>,
    on_shutdown: Option<ShutdownHook>,
    pm_hint: Arc<Mutex<Option<String>>>,
    events_tx: Option<broadcast::Sender<DaemonEvent>>,
}

impl IpcServer {
    pub fn new(registry: RegistryStore) -> Self {
        Self {
            registry,
            on_reconcile: None,
            on_shutdown: None,
            pm_hint: Arc::new(Mutex::new(None)),
            events_tx: None,
        }
    }

    pub fn with_pm_hint(mut self, hint: Arc<Mutex<Option<String>>>) -> Self {
        self.pm_hint = hint;
        self
    }

    pub fn with_reconcile_hook(mut self, hook: ReconcileHook) -> Self {
        self.on_reconcile = Some(hook);
        self
    }

    pub fn with_shutdown_hook(mut self, hook: ShutdownHook) -> Self {
        self.on_shutdown = Some(hook);
        self
    }

    pub fn with_events_tx(mut self, tx: broadcast::Sender<DaemonEvent>) -> Self {
        self.events_tx = Some(tx);
        self
    }

    pub async fn run(self) -> LinkdResult<()> {
        linkd_core::ensure_home()
            .map_err(|e| linkd_core::LinkdError::io(linkd_core::linkd_home(), e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            use tokio::net::UnixListener;

            let path = linkd_core::daemon_socket_path();
            if path.exists() {
                let _ = std::fs::remove_file(&path);
            }

            let listener =
                UnixListener::bind(&path).map_err(|e| linkd_core::LinkdError::io(&path, e))?;
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

            let pipe_name = linkd_core::daemon_pipe_name();
            let mut pipe = ServerOptions::new()
                .create(&pipe_name)
                .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;

            loop {
                pipe.connect()
                    .await
                    .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;

                let connected_pipe = pipe;
                pipe = ServerOptions::new()
                    .create(&pipe_name)
                    .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;

                let server = self.clone_inner();
                tokio::spawn(async move {
                    if let Err(e) = server.handle_connection_windows(connected_pipe).await {
                        error!("ipc pipe error: {e}");
                    }
                });
            }
        }
    }

    fn clone_inner(&self) -> Self {
        Self {
            registry: RegistryStore::new(self.registry.path().to_path_buf()),
            on_reconcile: self.on_reconcile.clone(),
            on_shutdown: self.on_shutdown.clone(),
            pm_hint: self.pm_hint.clone(),
            events_tx: self.events_tx.clone(),
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
            let req_res: Result<IpcRequest, _> = decode_line(&line);
            if let Ok(IpcRequest::SubscribeEvents { auth_token }) = req_res {
                if let Err(e) = verify_auth_token(&auth_token) {
                    let resp = IpcResponse::err(e.to_string());
                    let payload = encode_line(&resp)
                        .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
                    let _ = writer.write_all(payload.as_bytes()).await;
                    return Ok(());
                }

                if let Some(events_tx) = &self.events_tx {
                    let mut rx = events_tx.subscribe();
                    if let Ok(reg) = self.registry.load() {
                        let hint = self.pm_hint.lock().await.clone();
                        let initial = DaemonEvent::Snapshot {
                            snapshot: LinkStatusSnapshot {
                                links: reg.links,
                                daemon_running: true,
                                pm_install_hint: hint,
                            },
                        };
                        let payload = encode_line(&initial)
                            .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
                        if writer.write_all(payload.as_bytes()).await.is_err() {
                            return Ok(());
                        }
                    }

                    while let Ok(event) = rx.recv().await {
                        let payload = encode_line(&event)
                            .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
                        if writer.write_all(payload.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                    return Ok(());
                }
            }

            let resp = self.dispatch_line(&line).await;
            let payload =
                encode_line(&resp).map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
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
        pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    ) -> LinkdResult<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let (reader, mut writer) = tokio::io::split(pipe);
        let mut lines = BufReader::new(reader).lines();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?
        {
            let req_res: Result<IpcRequest, _> = decode_line(&line);
            if let Ok(IpcRequest::SubscribeEvents { auth_token }) = req_res {
                if let Err(e) = verify_auth_token(&auth_token) {
                    let resp = IpcResponse::err(e.to_string());
                    let payload = encode_line(&resp)
                        .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
                    let _ = writer.write_all(payload.as_bytes()).await;
                    return Ok(());
                }

                if let Some(events_tx) = &self.events_tx {
                    let mut rx = events_tx.subscribe();
                    if let Ok(reg) = self.registry.load() {
                        let hint = self.pm_hint.lock().await.clone();
                        let initial = DaemonEvent::Snapshot {
                            snapshot: LinkStatusSnapshot {
                                links: reg.links,
                                daemon_running: true,
                                pm_install_hint: hint,
                            },
                        };
                        let payload = encode_line(&initial)
                            .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
                        if writer.write_all(payload.as_bytes()).await.is_err() {
                            return Ok(());
                        }
                    }

                    while let Ok(event) = rx.recv().await {
                        let payload = encode_line(&event)
                            .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
                        if writer.write_all(payload.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                    return Ok(());
                }
            }

            let resp = self.dispatch_line(&line).await;
            let payload =
                encode_line(&resp).map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
            writer
                .write_all(payload.as_bytes())
                .await
                .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
        }
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
            | IpcRequest::TogglePauseLink { auth_token, .. }
            | IpcRequest::SubscribeEvents { auth_token, .. }
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
            IpcRequest::TogglePauseLink { package_name, .. } => {
                match self.registry.with_mut(|reg| {
                    let link = reg
                        .links
                        .iter_mut()
                        .find(|l| l.package_name == package_name)
                        .ok_or_else(|| {
                            linkd_core::LinkdError::PackageNotFound(package_name.clone())
                        })?;
                    let new_status = if link.last_sync_status == LinkSyncStatus::Paused {
                        LinkSyncStatus::Pending
                    } else {
                        LinkSyncStatus::Paused
                    };
                    link.last_sync_status = new_status;
                    let pkg_name = link.package_name.clone();
                    let src = link.source_path.clone();
                    let last_synced = link.last_sync_at;
                    Ok((new_status, pkg_name, src, last_synced))
                }) {
                    Ok((new_status, pkg_name, src, last_synced)) => {
                        if let Some(events_tx) = &self.events_tx {
                            let _ = events_tx.send(DaemonEvent::LinkStatusChanged {
                                package_name: pkg_name,
                                source: src,
                                status: new_status,
                                last_synced_at: last_synced,
                            });
                        }
                        if new_status == LinkSyncStatus::Pending {
                            if let Some(hook) = &self.on_reconcile {
                                hook(None);
                            }
                        }
                        IpcResponse::ok_message(format!("link status: {new_status:?}"))
                    }
                    Err(e) => IpcResponse::err(e.to_string()),
                }
            }
            IpcRequest::SubscribeEvents { .. } => IpcResponse::ok_message("subscribed"),
            IpcRequest::Shutdown { .. } => {
                info!("shutdown requested");
                if let Some(hook) = &self.on_shutdown {
                    hook();
                }
                IpcResponse::ok_message("shutting down")
            }
        }
    }
}
