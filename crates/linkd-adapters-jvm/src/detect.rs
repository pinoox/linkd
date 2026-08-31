use std::path::Path;

use linkd_core::{LinkdError, LinkdResult};

pub fn detect_jvm(source: &Path, consumer: &Path) -> bool {
    let source_is_jvm = source.join("pom.xml").is_file()
        || source.join("build.gradle").is_file()
        || source.join("build.gradle.kts").is_file()
        || source.join("settings.gradle").is_file()
        || source.join("settings.gradle.kts").is_file();

    let consumer_is_jvm = consumer.join("pom.xml").is_file()
        || consumer.join("build.gradle").is_file()
        || consumer.join("build.gradle.kts").is_file()
        || consumer.join("settings.gradle").is_file()
        || consumer.join("settings.gradle.kts").is_file();

    source_is_jvm || consumer_is_jvm
}

pub fn parse_jvm_package_name(source: &Path) -> LinkdResult<String> {
    let pom = source.join("pom.xml");
    if pom.is_file() {
        if let Ok(content) = std::fs::read_to_string(&pom) {
            let group = extract_xml_tag(&content, "groupId");
            let artifact = extract_xml_tag(&content, "artifactId");

            if let Some(art) = artifact {
                if let Some(grp) = group {
                    return Ok(format!("{grp}:{art}"));
                }
                return Ok(art);
            }
        }
    }

    let gradle = source.join("build.gradle");
    if gradle.is_file() {
        if let Ok(content) = std::fs::read_to_string(&gradle) {
            let group = extract_gradle_field(&content, "group");
            let root_name = source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "lib".into());

            if let Some(grp) = group {
                return Ok(format!("{grp}:{root_name}"));
            }
        }
    }

    let gradle_kts = source.join("build.gradle.kts");
    if gradle_kts.is_file() {
        if let Ok(content) = std::fs::read_to_string(&gradle_kts) {
            let group = extract_gradle_field(&content, "group");
            let root_name = source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "lib".into());

            if let Some(grp) = group {
                return Ok(format!("{grp}:{root_name}"));
            }
        }
    }

    source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| LinkdError::Other("could not determine JVM package name".into()))
}

fn extract_xml_tag(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if let Some(start) = content.find(&open) {
        let after = &content[start + open.len()..];
        if let Some(end) = after.find(&close) {
            let val = after[..end].trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn extract_gradle_field(content: &str, field: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(field) {
            let rest = rest.trim();
            let val = rest.strip_prefix('=').unwrap_or(rest).trim();
            let cleaned = val.trim_matches(|c| c == '\'' || c == '"');
            if !cleaned.is_empty() && cleaned != val {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}
