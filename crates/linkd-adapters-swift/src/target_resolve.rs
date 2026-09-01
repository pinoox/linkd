use std::path::Path;

use linkd_core::{IsolationMode, LinkdError, LinkdResult, ResolvedSyncTarget};

pub fn parse_package_name(source: &Path) -> LinkdResult<String> {
    let manifest = source.join("Package.swift");
    if !manifest.is_file() {
        return Err(LinkdError::PackageNotFound(
            "Package.swift not found".into(),
        ));
    }
    let content = std::fs::read_to_string(&manifest).map_err(|e| LinkdError::io(&manifest, e))?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if let Some(pos) = trimmed.find("name:") {
            let after = &trimmed[pos + 5..].trim();
            let mut chars = after.chars().peekable();
            if let Some(quote) = chars.next() {
                if quote == '"' || quote == '\'' {
                    let mut name = String::new();
                    for c in chars {
                        if c == quote {
                            break;
                        }
                        name.push(c);
                    }
                    if !name.is_empty() {
                        return Ok(name);
                    }
                }
            }
        }
    }

    source
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
        .ok_or_else(|| LinkdError::PackageNotFound("Swift package name in Package.swift".into()))
}

pub fn resolve_swift_target(
    consumer_root: &Path,
    package_name: &str,
) -> LinkdResult<ResolvedSyncTarget> {
    let target = consumer_root
        .join(".build")
        .join("checkouts")
        .join(package_name);

    Ok(ResolvedSyncTarget {
        logical_target: target.clone(),
        sync_target: target,
        isolation_mode: IsolationMode::ProjectLocal,
        forbidden_roots: vec![],
    })
}

pub fn detect_swift(source: &Path, consumer: &Path) -> bool {
    source.join("Package.swift").is_file()
        || consumer.join("Package.swift").is_file()
        || consumer.join("Package.resolved").is_file()
        || consumer.join(".build").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_package_swift_name() {
        let temp = tempfile::tempdir().unwrap();
        let pkg = temp.path().join("Package.swift");
        std::fs::write(
            &pkg,
            r#"// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SwiftUiKit",
    platforms: [.macOS(.v13), .iOS(.v16)],
    products: []
)"#,
        )
        .unwrap();

        let name = parse_package_name(temp.path()).unwrap();
        assert_eq!(name, "SwiftUiKit");
    }
}
