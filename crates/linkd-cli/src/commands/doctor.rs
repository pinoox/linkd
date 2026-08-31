use linkd_adapters_npm::{detect_package_manager, is_yarn_pnp, PackageManager, PnpmStoreDetector};
#[cfg(unix)]
use linkd_core::daemon_socket_path;
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
        println!("… Daemon not running (start with: linkd start or linkd watch)");
    }

    let reg = RegistryStore::default().load()?;
    for link in &reg.links {
        if is_yarn_pnp(&link.consumer_root) {
            println!(
                "✗ Yarn PnP detected in {} — switch to nodeLinker: node-modules",
                link.consumer_root.display()
            );
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

    let cwd = std::env::current_dir()?;
    let pm = detect_package_manager(cwd.as_path());
    let has_composer_json = cwd.join("composer.json").is_file();
    let composer_bin_available = is_executable_on_path("composer");

    if has_composer_json {
        if composer_bin_available {
            println!("✓ Detected Composer project (composer binary found on PATH)");
        } else {
            println!(
                "⚠ Detected Composer project (composer.json present, but `composer` not on PATH)"
            );
            issues += 1;
        }
    } else {
        println!("✓ Detected package manager: {}", pm_label(pm));
        if composer_bin_available {
            println!("✓ Composer CLI available on PATH");
        }
    }

    println!("\nHome: {}", linkd_home().display());

    if issues == 0 {
        println!("\nAll checks passed.");
    } else {
        println!("\nFound {issues} issue(s).");
    }

    Ok(())
}

fn is_executable_on_path(exe_name: &str) -> bool {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            #[cfg(windows)]
            {
                if dir.join(format!("{exe_name}.bat")).is_file()
                    || dir.join(format!("{exe_name}.exe")).is_file()
                    || dir.join(format!("{exe_name}.cmd")).is_file()
                {
                    return true;
                }
            }
            #[cfg(not(windows))]
            {
                if dir.join(exe_name).is_file() {
                    return true;
                }
            }
        }
    }
    false
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
        "composer" => {
            println!(
                "Composer ecosystem in linkd:\n\
                 - Links resolve to consumer/vendor/<vendor>/<package>\n\
                 - Watches vendor/composer/installed.json and classmap files\n\
                 - When new PHP classes are added to source, run:\n\
                     composer dump-autoload\n\
                   in the consumer project if classes are not detected."
            );
        }
        "python" => {
            println!(
                "Python ecosystem in linkd (uv / pip / poetry):\n\
                 - Links resolve to consumer/.venv/Lib/site-packages/<package> (or Unix lib/python*/...)\n\
                 - Watches uv.lock, poetry.lock, requirements.txt, pyvenv.cfg\n\
                 - Automatically filters __pycache__, *.pyc, .pytest_cache"
            );
        }
        "go" => {
            println!(
                "Go ecosystem in linkd (Go modules / vendor):\n\
                 - Links resolve to consumer/vendor/<module_path>\n\
                 - Watches go.sum, go.work.sum, go.work\n\
                 - Use `go build -mod=vendor` or Go workspaces (`go work`)"
            );
        }
        "cargo" | "rust" => {
            println!(
                "Rust / Cargo ecosystem in linkd:\n\
                 - Links resolve to consumer/vendor/<crate_name>\n\
                 - Watches Cargo.lock\n\
                 - Automatically filters target/ and build caches"
            );
        }
        "jvm" | "maven" | "gradle" => {
            println!(
                "JVM ecosystem in linkd (Maven / Gradle):\n\
                 - Links resolve to consumer/libs/<artifact> or Maven Local repo\n\
                 - Watches pom.xml, build.gradle, gradle.lockfile"
            );
        }
        "autostart" => {
            println!(
                "Background daemon:\n\
                   linkd start   — detached daemon (recommended)\n\
                   linkd watch   — foreground with live UI\n\
                 Auto-start on `linkd link` is enabled by default (~/.linkd/config.json).\n\n\
                 macOS launchd plist (~/Library/LaunchAgents/dev.linkd.daemon.plist):\n\
                 <?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
                 <plist version=\"1.0\">\n\
                 <dict>\n\
                   <key>Label</key><string>dev.linkd.daemon</string>\n\
                   <key>ProgramArguments</key>\n\
                   <array><string>/usr/local/bin/linkd</string><string>--daemon-internal</string></array>\n\
                   <key>RunAtLoad</key><true/>\n\
                   <key>KeepAlive</key><true/>\n\
                 </dict>\n\
                 </plist>\n\n\
                 Linux systemd user unit (~/.config/systemd/user/linkd.service):\n\
                 [Unit]\n\
                 Description=linkd local-dev link daemon\n\
                 After=default.target\n\n\
                 [Service]\n\
                 ExecStart=/usr/local/bin/linkd --daemon-internal\n\
                 Restart=always\n\n\
                 [Install]\n\
                 WantedBy=default.target"
            );
        }
        other => anyhow::bail!("unknown explain topic: {other}"),
    }
    Ok(())
}
