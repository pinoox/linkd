use std::path::PathBuf;

use linkd_adapters::completion_markers_for_link;

pub fn watch_marker_paths(links: &[linkd_core::LinkEntry]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for link in links {
        paths.extend(completion_markers_for_link(link));
        paths.push(link.source_path.clone());
    }
    paths.sort();
    paths.dedup();
    paths
}
