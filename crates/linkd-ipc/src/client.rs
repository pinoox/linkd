use linkd_core::{LinkdError, LinkdResult};

#[cfg(unix)]
use linkd_core::daemon_socket_path;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixStream;

use crate::auth::load_auth_token;
use crate::protocol::{decode_line, encode_line, IpcRequest, IpcResponse, LinkStatusSnapshot};

pub struct IpcClient {
    token: String,
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new().expect("auth token")
    }
}

impl IpcClient {
    pub fn new() -> LinkdResult<Self> {
        Ok(Self {
            token: load_auth_token()?,
        })
    }

    #[cfg(unix)]
    async fn connect(&self) -> LinkdResult<UnixStream> {
        let path = daemon_socket_path();
        UnixStream::connect(path)
            .await
            .map_err(|_| LinkdError::DaemonNotRunning)
    }

    #[cfg(windows)]
    async fn connect(&self) -> LinkdResult<tokio::net::windows::named_pipe::NamedPipeClient> {
        use tokio::net::windows::named_pipe::ClientOptions;
        ClientOptions::new()
            .open(linkd_core::daemon_pipe_name())
            .map_err(|_| LinkdError::DaemonNotRunning)
    }

    async fn request(&self, req: IpcRequest) -> LinkdResult<IpcResponse> {
        #[cfg(unix)]
        {
            let mut stream = self.connect().await?;
            let payload = encode_line(&req).map_err(|e| LinkdError::Other(e.to_string()))?;
            stream
                .write_all(payload.as_bytes())
                .await
                .map_err(|e| LinkdError::Other(e.to_string()))?;

            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| LinkdError::Other(e.to_string()))?;

            decode_line(&line).map_err(|e| LinkdError::Other(e.to_string()))
        }

        #[cfg(windows)]
        {
            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
            let pipe = self.connect().await?;
            let (reader, mut writer) = tokio::io::split(pipe);
            let payload = encode_line(&req).map_err(|e| LinkdError::Other(e.to_string()))?;
            writer
                .write_all(payload.as_bytes())
                .await
                .map_err(|e| LinkdError::Other(e.to_string()))?;

            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| LinkdError::Other(e.to_string()))?;
            decode_line(&line).map_err(|e| LinkdError::Other(e.to_string()))
        }
    }

    pub async fn ping(&self) -> LinkdResult<bool> {
        match self
            .request(IpcRequest::Ping {
                auth_token: self.token.clone(),
            })
            .await
        {
            Ok(IpcResponse::Ok { .. }) => Ok(true),
            Ok(IpcResponse::Error { .. }) => Ok(false),
            Err(LinkdError::DaemonNotRunning) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub async fn list_links(&self) -> LinkdResult<Vec<linkd_core::LinkEntry>> {
        let resp = self
            .request(IpcRequest::ListLinks {
                auth_token: self.token.clone(),
            })
            .await?;

        match resp {
            IpcResponse::Ok { links, .. } => Ok(links),
            IpcResponse::Error { message } => Err(LinkdError::Other(message)),
        }
    }

    pub async fn add_link(&self, entry: linkd_core::LinkEntry) -> LinkdResult<()> {
        let resp = self
            .request(IpcRequest::AddLink {
                auth_token: self.token.clone(),
                entry,
            })
            .await?;
        match resp {
            IpcResponse::Ok { .. } => Ok(()),
            IpcResponse::Error { message } => Err(LinkdError::Other(message)),
        }
    }

    pub async fn remove_link(&self, package_name: &str) -> LinkdResult<()> {
        let resp = self
            .request(IpcRequest::RemoveLink {
                auth_token: self.token.clone(),
                package_name: package_name.to_string(),
            })
            .await?;
        match resp {
            IpcResponse::Ok { .. } => Ok(()),
            IpcResponse::Error { message } => Err(LinkdError::Other(message)),
        }
    }

    pub async fn status(&self) -> LinkdResult<LinkStatusSnapshot> {
        let resp = self
            .request(IpcRequest::GetStatus {
                auth_token: self.token.clone(),
            })
            .await?;
        match resp {
            IpcResponse::Ok { snapshot, .. } => {
                snapshot.ok_or(LinkdError::Other("no snapshot".into()))
            }
            IpcResponse::Error { message } => Err(LinkdError::Other(message)),
        }
    }

    pub async fn trigger_reconcile(&self, link_id: Option<uuid::Uuid>) -> LinkdResult<()> {
        let resp = self
            .request(IpcRequest::TriggerReconcile {
                auth_token: self.token.clone(),
                link_id,
            })
            .await?;
        match resp {
            IpcResponse::Ok { .. } => Ok(()),
            IpcResponse::Error { message } => Err(LinkdError::Other(message)),
        }
    }

    pub async fn toggle_pause_link(&self, package_name: &str) -> LinkdResult<()> {
        let resp = self
            .request(IpcRequest::TogglePauseLink {
                auth_token: self.token.clone(),
                package_name: package_name.to_string(),
            })
            .await?;
        match resp {
            IpcResponse::Ok { .. } => Ok(()),
            IpcResponse::Error { message } => Err(LinkdError::Other(message)),
        }
    }

    pub async fn subscribe_events(
        &self,
    ) -> LinkdResult<tokio::sync::mpsc::Receiver<crate::DaemonEvent>> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let token = self.token.clone();

        #[cfg(unix)]
        {
            let mut stream = self.connect().await?;
            let req = IpcRequest::SubscribeEvents { auth_token: token };
            let payload = encode_line(&req).map_err(|e| LinkdError::Other(e.to_string()))?;
            stream
                .write_all(payload.as_bytes())
                .await
                .map_err(|e| LinkdError::Other(e.to_string()))?;

            tokio::spawn(async move {
                let (reader, _) = stream.into_split();
                let mut lines = BufReader::new(reader).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Ok(event) = decode_line::<crate::DaemonEvent>(&line) {
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                }
            });
        }

        #[cfg(windows)]
        {
            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
            let mut pipe = self.connect().await?;
            let req = IpcRequest::SubscribeEvents { auth_token: token };
            let payload = encode_line(&req).map_err(|e| LinkdError::Other(e.to_string()))?;
            pipe.write_all(payload.as_bytes())
                .await
                .map_err(|e| LinkdError::Other(e.to_string()))?;

            tokio::spawn(async move {
                let (reader, _) = tokio::io::split(pipe);
                let mut lines = BufReader::new(reader).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Ok(event) = decode_line::<crate::DaemonEvent>(&line) {
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                }
            });
        }

        Ok(rx)
    }

    pub async fn shutdown(&self) -> LinkdResult<()> {
        let resp = self
            .request(IpcRequest::Shutdown {
                auth_token: self.token.clone(),
            })
            .await?;
        match resp {
            IpcResponse::Ok { .. } => Ok(()),
            IpcResponse::Error { message } => Err(LinkdError::Other(message)),
        }
    }
}
