use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DebouncedEvent {
    pub key: String,
    pub paths: Vec<PathBuf>,
}

pub struct DebouncePool {
    delay: Duration,
    pending: HashMap<String, (Instant, Vec<PathBuf>)>,
}

impl DebouncePool {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay: Duration::from_millis(delay_ms),
            pending: HashMap::new(),
        }
    }

    pub fn push(&mut self, key: impl Into<String>, path: PathBuf) {
        let key = key.into();
        let entry = self.pending.entry(key.clone()).or_insert((Instant::now(), Vec::new()));
        entry.0 = Instant::now();
        entry.1.push(path);
    }

    pub fn ready(&mut self) -> Vec<DebouncedEvent> {
        let now = Instant::now();
        let mut ready = Vec::new();
        let mut done_keys = Vec::new();

        for (key, (last, paths)) in &self.pending {
            if now.duration_since(*last) >= self.delay {
                ready.push(DebouncedEvent {
                    key: key.clone(),
                    paths: paths.clone(),
                });
                done_keys.push(key.clone());
            }
        }

        for key in done_keys {
            self.pending.remove(&key);
        }

        ready
    }
}
