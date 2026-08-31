use std::process::{Command, Stdio};

use chrono::Utc;
use linkd_core::{DaemonPidFile, LinkdConfig, LinkdResult};

use crate::pid::{cleanup_stale_pid, is_daemon_running, is_linkd_process, read_daemon_pid};

#[cfg(windows)]
const DETACHED_FLAGS: u32 = 0x00000200 | 0x00000008;

pub fn ensure_daemon_running() -> LinkdResult<()> {
    let config = LinkdConfig::load()?;
    if !config.auto_start_daemon {
        return Ok(());
    }
    if is_daemon_running() {
        return Ok(());
    }
    start_daemon()?;
    Ok(())
}

pub fn start_daemon() -> LinkdResult<()> {
    cleanup_stale_pid()?;
    if is_daemon_running() {
        return Err(linkd_core::LinkdError::Other(
            "linkd daemon is already running".into(),
        ));
    }

    let exe = std::env::current_exe()
        .map_err(|e| linkd_core::LinkdError::Other(format!("cannot resolve linkd binary: {e}")))?;

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let child = Command::new(&exe)
            .arg("--daemon-internal")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(DETACHED_FLAGS)
            .spawn()
            .map_err(|e| linkd_core::LinkdError::Other(format!("failed to start daemon: {e}")))?;

        let pid = child.id();
        write_pid_file(pid)?;
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let child = Command::new(&exe)
            .arg("--daemon-internal")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| linkd_core::LinkdError::Other(format!("failed to start daemon: {e}")))?;

        let pid = child.id();
        write_pid_file(pid)?;
        Ok(())
    }
}

fn write_pid_file(pid: u32) -> LinkdResult<()> {
    DaemonPidFile {
        pid,
        started_at: Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
    .save()
}

pub fn stop_daemon(force: bool) -> LinkdResult<()> {
    let pid_file = match read_daemon_pid()? {
        Some(p) => p,
        None => return Ok(()),
    };

    if let Ok(client) = linkd_ipc::IpcClient::new() {
        let shutdown = tokio::runtime::Handle::try_current()
            .map(|h| h.block_on(client.shutdown()))
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.block_on(client.shutdown())));

        if let Ok(Ok(())) = shutdown {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if !is_linkd_process(pid_file.pid) {
                DaemonPidFile::remove()?;
                return Ok(());
            }
        }
    }

    if force || !is_linkd_process(pid_file.pid) {
        kill_process(pid_file.pid);
        DaemonPidFile::remove()?;
    } else if !force {
        return Err(linkd_core::LinkdError::Other(
            "daemon did not stop gracefully; use --force".into(),
        ));
    }

    Ok(())
}

fn kill_process(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }

    #[cfg(unix)]
    {
        let _ = Command::new("kill").arg(pid.to_string()).output();
    }
}

pub fn run_daemon_internal() -> LinkdResult<()> {
    cleanup_stale_pid()?;
    let pid = std::process::id();
    write_pid_file(pid)?;

    let result = tokio::runtime::Runtime::new()
        .map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?
        .block_on(async {
            let store = linkd_registry::RegistryStore::default();
            let service = crate::DaemonService::new(store);
            service.run_background().await
        });

    let _ = DaemonPidFile::remove();
    result
}
