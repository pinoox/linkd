use sysinfo::{ProcessRefreshKind, RefreshKind, System};

pub fn pm_install_hint(consumer_root: &std::path::Path) -> Option<String> {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_all();

    let names = ["npm", "pnpm", "yarn", "bun"];
    for (_pid, process) in system.processes() {
        let exe = process.exe()?.to_string_lossy().to_lowercase();
        if names.iter().any(|n| exe.contains(n)) {
            let cwd = process.cwd()?.to_string_lossy().to_string();
            if cwd.contains(&consumer_root.to_string_lossy().to_string()) {
                return Some(format!("install in progress ({exe})"));
            }
        }
    }
    None
}
