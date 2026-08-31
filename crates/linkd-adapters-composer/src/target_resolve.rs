use std::path::Path;

pub fn parse_package_name(source: &Path) -> linkd_core::LinkdResult<String> {
    let composer_json = source.join("composer.json");
    let data = std::fs::read_to_string(&composer_json)
        .map_err(|e| linkd_core::LinkdError::io(&composer_json, e))?;
    let v: serde_json::Value =
        serde_json::from_str(&data).map_err(|e| linkd_core::LinkdError::Other(e.to_string()))?;
    v.get("name")
        .and_then(|n| n.as_str())
        .map(String::from)
        .ok_or_else(|| linkd_core::LinkdError::Other("composer.json missing name".into()))
}

pub fn resolve_vendor_target(
    consumer_root: &Path,
    package_name: &str,
) -> linkd_core::LinkdResult<linkd_core::ResolvedSyncTarget> {
    let parts: Vec<&str> = package_name.split('/').collect();
    if parts.len() != 2 {
        return Err(linkd_core::LinkdError::Other(format!(
            "invalid composer package name: {package_name}"
        )));
    }

    let target = consumer_root.join("vendor").join(parts[0]).join(parts[1]);

    Ok(linkd_core::ResolvedSyncTarget {
        logical_target: target.clone(),
        sync_target: target,
        isolation_mode: linkd_core::IsolationMode::ProjectLocal,
        forbidden_roots: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolves_vendor_path() {
        let tmp = TempDir::new().unwrap();
        let consumer = tmp.path().join("app");
        std::fs::create_dir_all(&consumer).unwrap();

        let resolved = resolve_vendor_target(&consumer, "acme/php-lib").unwrap();
        assert_eq!(
            resolved.sync_target,
            consumer.join("vendor").join("acme").join("php-lib")
        );
    }
}
