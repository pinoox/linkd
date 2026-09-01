use std::path::Path;

use linkd_core::{IsolationMode, LinkdError, LinkdResult, ResolvedSyncTarget};

pub fn parse_package_name(source: &Path) -> LinkdResult<String> {
    if let Ok(entries) = std::fs::read_dir(source) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext == "gemspec" {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with('#') {
                                continue;
                            }
                            if let Some(pos) = trimmed.find(".name") {
                                if let Some(eq_pos) = trimmed[pos..].find('=') {
                                    let val_part = &trimmed[pos + eq_pos + 1..].trim();
                                    let name = val_part
                                        .trim_matches('"')
                                        .trim_matches('\'')
                                        .trim_matches(':')
                                        .trim();
                                    if !name.is_empty() {
                                        return Ok(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        return Ok(stem.to_string());
                    }
                }
            }
        }
    }

    source
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
        .ok_or_else(|| LinkdError::PackageNotFound("Ruby gem (.gemspec)".into()))
}

pub fn resolve_ruby_target(
    consumer_root: &Path,
    package_name: &str,
) -> LinkdResult<ResolvedSyncTarget> {
    let target = consumer_root
        .join("vendor")
        .join("bundle")
        .join("gems")
        .join(package_name);

    Ok(ResolvedSyncTarget {
        logical_target: target.clone(),
        sync_target: target,
        isolation_mode: IsolationMode::ProjectLocal,
        forbidden_roots: vec![],
    })
}

pub fn detect_ruby(source: &Path, consumer: &Path) -> bool {
    has_ruby_manifest(source) || has_ruby_manifest(consumer)
}

fn has_ruby_manifest(dir: &Path) -> bool {
    if dir.join("Gemfile").is_file() || dir.join("Rakefile").is_file() {
        return true;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                if ext == "gemspec" {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gemspec_name() {
        let temp = tempfile::tempdir().unwrap();
        let gemspec = temp.path().join("my_gem.gemspec");
        std::fs::write(
            &gemspec,
            r#"Gem::Specification.new do |spec|
  spec.name = "acme-ruby-logger"
  spec.version = "1.0.0"
end"#,
        )
        .unwrap();

        let name = parse_package_name(temp.path()).unwrap();
        assert_eq!(name, "acme-ruby-logger");
    }
}
