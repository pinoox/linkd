use std::path::Path;

use linkd_core::{LinkdError, LinkdResult};

pub fn detect_python(source: &Path, consumer: &Path) -> bool {
    let source_is_python = source.join("pyproject.toml").is_file()
        || source.join("setup.py").is_file()
        || source.join("setup.cfg").is_file();

    let consumer_is_python = consumer.join(".venv").exists()
        || consumer.join("venv").exists()
        || consumer.join("pyproject.toml").is_file()
        || consumer.join("Pipfile").is_file()
        || consumer.join("requirements.txt").is_file();

    source_is_python || consumer_is_python
}

pub fn parse_package_name(source: &Path) -> LinkdResult<String> {
    let pyproject = source.join("pyproject.toml");
    if pyproject.is_file() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            let mut current_section = "";
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    current_section = trimmed[1..trimmed.len() - 1].trim();
                    continue;
                }
                if current_section == "project"
                    || current_section == "tool.poetry"
                    || current_section == "tool.flit.metadata"
                {
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

    let setup_cfg = source.join("setup.cfg");
    if setup_cfg.is_file() {
        if let Ok(content) = std::fs::read_to_string(&setup_cfg) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("name") {
                    let rest = rest.trim();
                    if let Some(name) = rest.strip_prefix('=') {
                        let name = name.trim();
                        if !name.is_empty() {
                            return Ok(name.to_string());
                        }
                    }
                }
            }
        }
    }

    let setup_py = source.join("setup.py");
    if setup_py.is_file() {
        if let Ok(content) = std::fs::read_to_string(&setup_py) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(pos) = trimmed.find("name=") {
                    let rest = &trimmed[pos + 5..].trim();
                    if (rest.starts_with('"') || rest.starts_with('\'')) && rest.len() > 2 {
                        let quote = rest.chars().next().unwrap();
                        if let Some(end) = rest[1..].find(quote) {
                            return Ok(rest[1..1 + end].to_string());
                        }
                    }
                }
            }
        }
    }

    source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| LinkdError::Other("could not determine python package name".into()))
}
