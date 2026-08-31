use std::path::Path;

use linkd_core::{LinkdError, LinkdResult};

pub fn detect_go(source: &Path, consumer: &Path) -> bool {
    let source_is_go = source.join("go.mod").is_file();
    let consumer_is_go = consumer.join("go.mod").is_file() || consumer.join("go.work").is_file();
    source_is_go || consumer_is_go
}

pub fn parse_module_name(source: &Path) -> LinkdResult<String> {
    let go_mod = source.join("go.mod");
    if !go_mod.is_file() {
        return source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| {
                LinkdError::Other("missing go.mod and cannot infer module name".into())
            });
    }

    let content = std::fs::read_to_string(&go_mod).map_err(|e| LinkdError::io(&go_mod, e))?;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("module") {
            let module = rest.trim();
            if !module.is_empty() {
                return Ok(module.to_string());
            }
        }
    }

    source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| LinkdError::Other("invalid go.mod without module declaration".into()))
}
