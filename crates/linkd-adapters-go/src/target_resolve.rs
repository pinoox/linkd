use std::path::Path;

use linkd_core::{IsolationMode, LinkdResult, ResolvedSyncTarget};

pub fn resolve_go_target(
    consumer_root: &Path,
    module_name: &str,
) -> LinkdResult<ResolvedSyncTarget> {
    let mut target = consumer_root.join("vendor");
    for part in module_name.split('/') {
        if !part.is_empty() {
            target = target.join(part);
        }
    }

    Ok(ResolvedSyncTarget {
        logical_target: target.clone(),
        sync_target: target,
        isolation_mode: IsolationMode::ProjectLocal,
        forbidden_roots: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolves_go_vendor_path() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let resolved = resolve_go_target(&consumer, "github.com/example/lib").unwrap();
        assert_eq!(
            resolved.sync_target,
            consumer
                .join("vendor")
                .join("github.com")
                .join("example")
                .join("lib")
        );
    }
}
