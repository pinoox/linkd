use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use linkd_core::{DaemonPidFile, LinkdResult};

pub fn is_linkd_process(pid: u32) -> bool {
    let mut system = System::new();
    let sys_pid = Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[sys_pid]),
        true,
        ProcessRefreshKind::new()
            .with_exe(UpdateKind::Always)
            .with_cmd(UpdateKind::Always),
    );

    let Some(process) = system.process(sys_pid) else {
        return false;
    };

    let name = process.name().to_string_lossy().to_lowercase();
    let exe = process
        .exe()
        .map(|p| p.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name.contains("linkd") || exe.contains("linkd")
}

pub fn is_daemon_running() -> bool {
    let _ = cleanup_stale_pid();
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
