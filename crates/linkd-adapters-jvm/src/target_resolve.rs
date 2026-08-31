use std::path::Path;

use linkd_core::{IsolationMode, LinkdResult, ResolvedSyncTarget};

pub fn resolve_jvm_target(
    consumer_root: &Path,
    package_name: &str,
) -> LinkdResult<ResolvedSyncTarget> {
    let artifact = if let Some((_, art)) = package_name.split_once(':') {
        art
    } else {
        package_name
    };

    let target = consumer_root.join("libs").join(artifact);

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
    fn resolves_jvm_libs_path() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        let resolved = resolve_jvm_target(&consumer, "com.acme:java-lib").unwrap();
        assert_eq!(resolved.sync_target, consumer.join("libs").join("java-lib"));
    }
}
