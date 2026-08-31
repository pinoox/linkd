use linkd_core::{DaemonPidFile, LinkdResult};
use linkd_daemon::pid::{cleanup_stale_pid, is_daemon_running, is_linkd_process};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn stale_pid_cleanup_removes_non_linkd_process() {
    let _guard = ENV_LOCK.lock().unwrap();
    let tmp = tempfile::TempDir::new().unwrap();
    std::env::set_var("LINKD_HOME", tmp.path());

    DaemonPidFile {
        pid: 999_999,
        started_at: "2026-01-01T00:00:00Z".into(),
        version: "0.1.0".into(),
    }
    .save()
    .unwrap();

    assert!(!is_linkd_process(999_999));
    cleanup_stale_pid().unwrap();
    assert!(DaemonPidFile::load().unwrap().is_none());

    std::env::remove_var("LINKD_HOME");
}

#[test]
fn pid_file_roundtrip() -> LinkdResult<()> {
    let _guard = ENV_LOCK.lock().unwrap();
    let tmp = tempfile::TempDir::new().unwrap();
    std::env::set_var("LINKD_HOME", tmp.path());

    let file = DaemonPidFile {
        pid: std::process::id(),
        started_at: "2026-01-01T00:00:00Z".into(),
        version: "0.1.0".into(),
    };
    file.save()?;
    let loaded = DaemonPidFile::load()?.expect("pid file");
    assert_eq!(loaded.pid, file.pid);

    DaemonPidFile::remove()?;
    assert!(DaemonPidFile::load()?.is_none());

    std::env::remove_var("LINKD_HOME");
    Ok(())
}

#[test]
fn daemon_running_check_identifies_current_process() -> LinkdResult<()> {
    let _guard = ENV_LOCK.lock().unwrap();
    let tmp = tempfile::TempDir::new().unwrap();
    std::env::set_var("LINKD_HOME", tmp.path());

    // Without PID file, daemon is not running
    assert!(!is_daemon_running());

    // With a non-linkd PID, is_daemon_running returns false
    DaemonPidFile {
        pid: 999_999,
        started_at: "2026-01-01T00:00:00Z".into(),
        version: "0.1.0".into(),
    }
    .save()?;
    assert!(!is_daemon_running());

    std::env::remove_var("LINKD_HOME");
    Ok(())
}
