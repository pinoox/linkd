use std::path::{Path, PathBuf};

pub fn parse_package_name(source: &Path) -> linkd_core::LinkdResult<String> {
    let pkg_json = source.join("package.json");
    let data = std::fs::read_to_string(&pkg_json).map_err(|e| linkd_core::LinkdError::io(&pkg_json, e))?;
    let v: serde_json::Value =
        serde_json::from_str(&data).map_err(|e| linkd_core::LinkdError::NpmPackFailed(e.to_string()))?;
    v.get("name")
        .and_then(|n| n.as_str())
        .map(String::from)
        .ok_or_else(|| linkd_core::LinkdError::NpmPackFailed("package.json missing name".into()))
}

pub fn resolve_node_modules_target(consumer_root: &Path, package_name: &str) -> PathBuf {
    let mut target = consumer_root.join("node_modules");
    if package_name.starts_with('@') {
        if let Some((scope, name)) = package_name.split_once('/') {
            target.push(scope);
            target.push(name);
            return target;
        }
    }
    target.push(package_name);
    target
}

pub fn shadow_target_path(consumer_root: &Path, package_name: &str) -> PathBuf {
    linkd_sync::shadow_dir(consumer_root, package_name)
}
