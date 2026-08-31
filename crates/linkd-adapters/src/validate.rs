use std::path::Path;

use linkd_core::paths::normalize_path;
use linkd_core::{LinkdError, LinkdResult};

pub fn validate_link_paths(source: &Path, target: &Path) -> LinkdResult<()> {
    let src = normalize_path(source);
    let tgt = normalize_path(target);

    #[cfg(windows)]
    let (src_str, tgt_str) = {
        (
            src.to_string_lossy().to_lowercase().replace('/', "\\"),
            tgt.to_string_lossy().to_lowercase().replace('/', "\\"),
        )
    };
    #[cfg(not(windows))]
    let (src_str, tgt_str) = {
        (
            src.to_string_lossy().to_string(),
            tgt.to_string_lossy().to_string(),
        )
    };

    if src_str == tgt_str {
        return Err(LinkdError::Other(
            "source and target must not be the same path".into(),
        ));
    }

    let src_prefix = if src_str.ends_with(std::path::MAIN_SEPARATOR) {
        src_str.clone()
    } else {
        format!("{}{}", src_str, std::path::MAIN_SEPARATOR)
    };

    let tgt_prefix = if tgt_str.ends_with(std::path::MAIN_SEPARATOR) {
        tgt_str.clone()
    } else {
        format!("{}{}", tgt_str, std::path::MAIN_SEPARATOR)
    };

    if tgt_str.starts_with(&src_prefix) || src_str.starts_with(&tgt_prefix) {
        return Err(LinkdError::Other(
            "source and target must not be nested (would cause watch loop)".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn rejects_nested_paths() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path().join("parent");
        let child = parent.join("child");
        std::fs::create_dir_all(&child).unwrap();

        assert!(validate_link_paths(&parent, &child).is_err());
        assert!(validate_link_paths(&child, &parent).is_err());
    }

    #[test]
    fn allows_sibling_paths() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();

        assert!(validate_link_paths(&a, &b).is_ok());
    }
}
