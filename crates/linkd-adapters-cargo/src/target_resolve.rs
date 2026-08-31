use std::path::Path;

use linkd_core::{IsolationMode, LinkdResult, ResolvedSyncTarget};

pub fn resolve_cargo_target(
    consumer_root: &Path,
    crate_name: &str,
) -> LinkdResult<ResolvedSyncTarget> {
    let target = consumer_root.join("vendor").join(crate_name);

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
    fn resolves_cargo_vendor_path() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let resolved = resolve_cargo_target(&consumer, "my-crate").unwrap();
        assert_eq!(
            resolved.sync_target,
            consumer.join("vendor").join("my-crate")
        );
    }
}
