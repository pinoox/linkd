use std::path::Path;

use linkd_core::{IsolationMode, LinkdError, LinkdResult, ResolvedSyncTarget};

pub fn parse_package_name(source: &Path) -> LinkdResult<String> {
    let pubspec = source.join("pubspec.yaml");
    if !pubspec.is_file() {
        return Err(LinkdError::PackageNotFound("pubspec.yaml not found".into()));
    }
    let content = std::fs::read_to_string(&pubspec).map_err(|e| LinkdError::io(&pubspec, e))?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name:") {
            let name = rest.trim().trim_matches('"').trim_matches('\'').trim();
            if !name.is_empty() {
                return Ok(name.to_string());
            }
        }
    }

    Err(LinkdError::PackageNotFound(
        "name field in pubspec.yaml".into(),
    ))
}

pub fn resolve_dart_target(
    consumer_root: &Path,
    package_name: &str,
) -> LinkdResult<ResolvedSyncTarget> {
    let target = consumer_root
        .join(".dart_tool")
        .join("packages")
        .join(package_name);

    Ok(ResolvedSyncTarget {
        logical_target: target.clone(),
        sync_target: target,
        isolation_mode: IsolationMode::ProjectLocal,
        forbidden_roots: vec![],
    })
}

pub fn detect_dart(source: &Path, consumer: &Path) -> bool {
    source.join("pubspec.yaml").is_file()
        || consumer.join("pubspec.yaml").is_file()
        || consumer.join(".dart_tool").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pubspec_name() {
        let temp = tempfile::tempdir().unwrap();
        let pubspec = temp.path().join("pubspec.yaml");
        std::fs::write(
            &pubspec,
            "name: my_flutter_package\ndescription: A test package\nversion: 1.0.0\n",
        )
        .unwrap();

        let name = parse_package_name(temp.path()).unwrap();
        assert_eq!(name, "my_flutter_package");
    }
}
