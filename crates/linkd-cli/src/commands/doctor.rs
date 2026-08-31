use linkd_adapters_npm::{
    detect_package_manager, is_yarn_pnp, PackageManager, PnpmStoreDetector,
};
use linkd_core::{is_ci, linkd_home};
use linkd_ipc::IpcClient;
use linkd_registry::RegistryStore;

pub async fn run(explain: Option<&str>) -> anyhow::Result<()> {
    if let Some(topic) = explain {
        return explain_topic(topic);
    }

    let mut issues = 0u32;

    println!("linkd doctor\n");

    if is_ci() {
        println!("⚠  CI=true detected — linkd is for local dev only.");
        issues += 1;
    }

    let daemon_ok = if let Ok(client) = IpcClient::new() {
        client.ping().await.unwrap_or(false)
    } else {
        false
    };

    if daemon_ok {
        println!("✓ Daemon running");
    } else {
        println!("… Daemon not running (start with: linkd watch)");
    }

    let reg = RegistryStore::default().load()?;
    for link in &reg.links {
        if is_yarn_pnp(&link.consumer_root) {
            println!("✗ Yarn PnP detected in {} — switch to nodeLinker: node-modules", link.consumer_root.display());
            issues += 1;
        }

        if !link.source_path.exists() {
            println!("✗ Stale link: source missing for {}", link.package_name);
            issues += 1;
        }

        if PnpmStoreDetector::is_global_store_path(&link.sync_target) {
            println!(
                "✗ BUG: link {} sync_target is inside pnpm global store!",
                link.package_name
            );
            issues += 1;
        }
    }

    #[cfg(unix)]
    {
        let sock = daemon_socket_path();
        if sock.exists() {
            let meta = std::fs::metadata(&sock)?;
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode() & 0o777;
            if mode != 0o600 {
                println!("⚠  IPC socket permissions {mode:o} (expected 0600)");
                issues += 1;
            } else {
                println!("✓ IPC socket permissions 0600");
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(v) = std::fs::read_to_string("/proc/sys/fs/inotify/max_user_watches") {
            let n: u64 = v.trim().parse().unwrap_or(0);
            if n < 524_288 {
                println!("⚠  inotify max_user_watches={n} (may be low for large projects)");
            } else {
                println!("✓ inotify max_user_watches={n}");
            }
        }
    }

    let pm = detect_package_manager(std::env::current_dir()?.as_path());
    println!("✓ Detected package manager: {}", pm_label(pm));

    println!("\nHome: {}", linkd_home().display());

    if issues == 0 {
        println!("\nAll checks passed.");
    } else {
        println!("\nFound {issues} issue(s).");
    }

    Ok(())
}

fn pm_label(pm: PackageManager) -> &'static str {
    match pm {
        PackageManager::Npm => "npm",
        PackageManager::Pnpm => "pnpm",
        PackageManager::Yarn => "yarn",
        PackageManager::Bun => "bun",
        PackageManager::Unknown => "unknown",
    }
}

fn explain_topic(topic: &str) -> anyhow::Result<()> {
    match topic {
        "pnpm-store" => {
            println!(
                "pnpm stores packages in a global content-addressable store.\n\
                 linkd NEVER writes there directly.\n\
                 When needed, it creates an isolated copy under:\n\
                   node_modules/.linkd-shadow/<package>\n\
                 and repoints the project symlink to that shadow copy."
            );
        }
        other => anyhow::bail!("unknown explain topic: {other}"),
    }
    Ok(())
}
