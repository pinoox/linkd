use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

use linkd_core::{DaemonPidFile, LinkdResult};

pub fn is_linkd_process(pid: u32) -> bool {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_all();

    let Some(process) = system.process(Pid::from_u32(pid)) else {
        return false;
    };

    let name = process.name().to_string_lossy().to_lowercase();
    name.contains("linkd")
}

pub fn is_daemon_running() -> bool {
    match DaemonPidFile::load() {
        Ok(Some(pid_file)) => is_linkd_process(pid_file.pid),
        _ => false,
    }
}

pub fn cleanup_stale_pid() -> LinkdResult<()> {
    if let Ok(Some(pid_file)) = DaemonPidFile::load() {
        if !is_linkd_process(pid_file.pid) {
            DaemonPidFile::remove()?;
        }
    }
    Ok(())
}

pub fn read_daemon_pid() -> LinkdResult<Option<DaemonPidFile>> {
    cleanup_stale_pid()?;
    DaemonPidFile::load()
}
