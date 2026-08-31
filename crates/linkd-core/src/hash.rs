use std::path::Path;

use crate::types::hash_files;

pub fn content_hash(source_root: &Path, files: &[std::path::PathBuf]) -> String {
    hash_files(source_root, files)
}
