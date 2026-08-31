use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone)]
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
    _watcher: RecommendedWatcher,
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

        for path in &watch_paths {
            if path.exists() {
                let mode = if path.is_dir() {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                };
                let _ = watcher.watch(path, mode);
            }
        }

        std::thread::spawn(move || {
            while let Ok(res) = raw_rx.recv() {
                if let Ok(event) = res {
                    if let Some(converted) = classify_event(&event) {
                        let _ = event_tx.send(converted);
                    }
                }
            }
        });

        Ok((Self { _watcher: watcher }, event_rx))
    }
}

fn classify_event(event: &Event) -> Option<WatchEvent> {
    let path = event.paths.first()?.clone();
    let kind = match event.kind {
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
            if path.to_string_lossy().contains(".linkd-marker") {
                WatchEventKind::TargetChanged
            } else if path.to_string_lossy().contains(".modules.yaml")
                || path.to_string_lossy().contains(".package-lock.json")
                || path.to_string_lossy().contains(".yarn-integrity")
            {
                WatchEventKind::MarkerChanged
            } else if path.to_string_lossy().contains("node_modules") {
                WatchEventKind::TargetChanged
            } else {
                WatchEventKind::SourceChanged
            }
        }
        _ => return None,
    };

    Some(WatchEvent {
        kind,
        path,
        consumer_root: None,
    })
}
