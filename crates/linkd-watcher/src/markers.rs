use std::path::{Path, PathBuf};

use linkd_adapters_npm::{detect_package_manager, completion_markers};

pub fn completion_markers_for_consumer(consumer_root: &Path) -> Vec<PathBuf> {
    let pm = detect_package_manager(consumer_root);
    completion_markers(consumer_root, pm)
}

pub fn watch_marker_paths(links: &[linkd_core::LinkEntry]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for link in links {
        paths.extend(completion_markers_for_consumer(&link.consumer_root));
        paths.push(link.source_path.clone());
        paths.push(link.sync_target.clone());
    }
    paths.sort();
    paths.dedup();
    paths
}
