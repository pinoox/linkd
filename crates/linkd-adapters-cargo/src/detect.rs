use std::path::Path;

use linkd_core::{LinkdError, LinkdResult};

pub fn detect_cargo(source: &Path, consumer: &Path) -> bool {
    source.join("Cargo.toml").is_file() || consumer.join("Cargo.toml").is_file()
}

pub fn parse_crate_name(source: &Path) -> LinkdResult<String> {
    let cargo_toml = source.join("Cargo.toml");
    if cargo_toml.is_file() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            let mut current_section = "";
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    current_section = trimmed[1..trimmed.len() - 1].trim();
                    continue;
                }
                if current_section == "package" {
                    if let Some(rest) = trimmed.strip_prefix("name") {
                        let rest = rest.trim();
                        if let Some(val) = rest.strip_prefix('=') {
                            let val = val.trim().trim_matches(|c| c == '\'' || c == '"');
                            if !val.is_empty() {
                                return Ok(val.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| LinkdError::Other("could not determine Cargo crate name".into()))
}
