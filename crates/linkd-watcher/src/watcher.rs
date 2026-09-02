use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEventKind {
    SourceChanged,
    TargetChanged,
    MarkerChanged,
}

#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub kind: WatchEventKind,
    pub path: PathBuf,
    pub consumer_root: Option<PathBuf>,
}

pub struct LinkWatcher {
    watcher: RecommendedWatcher,
    watched_paths: HashSet<PathBuf>,
}

impl LinkWatcher {
    pub fn new(watch_paths: Vec<PathBuf>) -> notify::Result<(Self, Receiver<WatchEvent>)> {
        let (raw_tx, raw_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = raw_tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_millis(300)),
        )?;

        let mut watched_paths = HashSet::new();
        for path in watch_paths {
            let normalized = linkd_core::normalize_path(&path);
            if normalized.exists() && !watched_paths.contains(&normalized) {
                let mode = if normalized.is_dir() {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                };
                if watcher.watch(&normalized, mode).is_ok() {
                    watched_paths.insert(normalized);
                }
            }
        }

        std::thread::spawn(move || {
            while let Ok(res) = raw_rx.recv() {
                if let Ok(event) = res {
                    for converted in classify_events(&event) {
                        let _ = event_tx.send(converted);
                    }
                }
            }
        });

        Ok((
            Self {
                watcher,
                watched_paths,
            },
            event_rx,
        ))
    }

    pub fn watch_path(&mut self, path: &Path) -> notify::Result<bool> {
        let normalized = linkd_core::normalize_path(path);
        if !normalized.exists() {
            return Ok(false);
        }
        if self.watched_paths.contains(&normalized) {
            return Ok(false);
        }
        let mode = if normalized.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        self.watcher.watch(&normalized, mode)?;
        self.watched_paths.insert(normalized);
        Ok(true)
    }

    pub fn unwatch_path(&mut self, path: &Path) -> notify::Result<bool> {
        let normalized = linkd_core::normalize_path(path);
        if self.watched_paths.remove(&normalized) {
            let _ = self.watcher.unwatch(&normalized);
            return Ok(true);
        }
        Ok(false)
    }

    pub fn sync_paths(&mut self, desired_paths: &[PathBuf]) {
        let desired_set: HashSet<PathBuf> = desired_paths
            .iter()
            .map(|p| linkd_core::normalize_path(p))
            .filter(|p| p.exists())
            .collect();

        if self.watched_paths == desired_set {
            return;
        }

        let current = self.watched_paths.clone();
        for p in current {
            if !desired_set.contains(&p) {
                let _ = self.unwatch_path(&p);
            }
        }

        for p in desired_set {
            if !self.watched_paths.contains(&p) {
                let _ = self.watch_path(&p);
            }
        }
    }

    pub fn watched_paths(&self) -> &HashSet<PathBuf> {
        &self.watched_paths
    }
}

fn classify_events(event: &Event) -> Vec<WatchEvent> {
    if matches!(event.kind, EventKind::Access(_)) {
        return Vec::new();
    }

    let mut events = Vec::new();
    for path in &event.paths {
        let path_str = path.to_string_lossy().to_lowercase();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let kind = if file_name == ".linkd-marker.json"
            || path_str.contains(".linkd-marker")
            || path_str.contains(".linkd-shadow")
        {
            WatchEventKind::TargetChanged
        } else if is_lockfile_or_reinstall_marker(&file_name, &path_str) {
            WatchEventKind::MarkerChanged
        } else if is_known_target_directory(&path_str) {
            WatchEventKind::TargetChanged
        } else {
            WatchEventKind::SourceChanged
        };

        events.push(WatchEvent {
            kind,
            path: path.clone(),
            consumer_root: None,
        });
    }
    events
}

fn is_lockfile_or_reinstall_marker(file_name: &str, path_str: &str) -> bool {
    file_name == "package-lock.json"
        || file_name == ".package-lock.json"
        || file_name == "pnpm-lock.yaml"
        || file_name == "pnpm-workspace.yaml"
        || file_name == ".modules.yaml"
        || file_name == "yarn.lock"
        || file_name == ".yarn-integrity"
        || file_name == "bun.lockb"
        || file_name == ".bun-tag"
        || file_name == "composer.lock"
        || file_name == "uv.lock"
        || file_name == "poetry.lock"
        || file_name == "pipfile.lock"
        || file_name == "requirements.txt"
        || file_name == "pyvenv.cfg"
        || file_name == "go.sum"
        || file_name == "go.work.sum"
        || file_name == "cargo.lock"
        || file_name == "pubspec.lock"
        || file_name == "packages.lock.json"
        || file_name == "gemfile.lock"
        || file_name == "package.resolved"
        || file_name == "mix.lock"
        || path_str.contains(".package-lock.json")
        || path_str.contains(".modules.yaml")
        || path_str.contains(".yarn-integrity")
}

fn is_known_target_directory(path_str: &str) -> bool {
    path_str.contains("node_modules")
        || path_str.contains(".venv")
        || path_str.contains("/venv/")
        || path_str.contains("\\venv\\")
        || path_str.contains("/.dart_tool/")
        || path_str.contains("\\.dart_tool\\")
        || path_str.contains("/.build/")
        || path_str.contains("\\.build\\")
}
