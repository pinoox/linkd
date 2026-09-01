use std::path::Path;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

pub fn pm_install_hint(consumer_root: &Path) -> Option<String> {
    let system = System::new_with_specifics(
        RefreshKind::new().with_processes(
            ProcessRefreshKind::new()
                .with_exe(UpdateKind::Always)
                .with_cwd(UpdateKind::Always),
        ),
    );

    let names = [
        "npm", "pnpm", "yarn", "bun", "composer", "cargo", "pip", "uv", "poetry", "flutter",
        "dart", "mix", "bundle", "dotnet", "gradle",
    ];
    let clean_consumer = linkd_core::clean_path(consumer_root)
        .to_string_lossy()
        .to_lowercase();

    for process in system.processes().values() {
        if let Some(exe_path) = process.exe() {
            let exe = exe_path.to_string_lossy().to_lowercase();
            if names.iter().any(|n| exe.contains(n)) {
                if let Some(cwd_path) = process.cwd() {
                    let cwd = linkd_core::clean_path(cwd_path)
                        .to_string_lossy()
                        .to_lowercase();
                    if cwd.contains(&clean_consumer) || clean_consumer.contains(&cwd) {
                        let pm_name = names
                            .iter()
                            .find(|n| exe.contains(**n))
                            .unwrap_or(&"package manager");
                        return Some(format!("install in progress ({pm_name})"));
                    }
                }
            }
        }
    }
    None
}
