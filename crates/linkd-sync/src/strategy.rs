use std::fs;
use std::io;
use std::path::Path;

use filetime::{set_file_mtime, FileTime};
use linkd_core::SyncStrategy;
use sha2::{Digest, Sha256};

/// Compares a source file against a destination file.
/// Layer 1: Fast metadata check (size & mtime).
/// Layer 2: Content hash comparison if mtime differs.
pub fn is_file_identical(source: &Path, dest: &Path) -> bool {
    let Ok(src_meta) = fs::metadata(source) else {
        return false;
    };
    let Ok(dst_meta) = fs::metadata(dest) else {
        return false;
    };

    if !src_meta.is_file() || !dst_meta.is_file() {
        return false;
    }

    // Layer 1a: File size check
    if src_meta.len() != dst_meta.len() {
        return false;
    }

    // Layer 1b: Fast timestamp match
    if let (Ok(src_time), Ok(dst_time)) = (src_meta.modified(), dst_meta.modified()) {
        if src_time == dst_time {
            return true;
        }
    }

    // Layer 2: Fast SHA-256 hash comparison if timestamps differ
    let Ok(src_bytes) = fs::read(source) else {
        return false;
    };
    let Ok(dst_bytes) = fs::read(dest) else {
        return false;
    };

    let src_hash = Sha256::digest(&src_bytes);
    let dst_hash = Sha256::digest(&dst_bytes);

    if src_hash == dst_hash {
        // Synchronize mtime so future checks hit Layer 1 instant cache
        let _ = set_file_mtime(dest, FileTime::from_last_modification_time(&src_meta));
        return true;
    }

    false
}

pub fn copy_file_with_strategy(
    source: &Path,
    dest: &Path,
    strategy: SyncStrategy,
) -> io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    match strategy {
        SyncStrategy::Hardlink => match fs::hard_link(source, dest) {
            Ok(()) => Ok(()),
            Err(_) => fs::copy(source, dest).map(|_| ()),
        },
        SyncStrategy::Symlink => {
            #[cfg(unix)]
            {
                let _ = fs::remove_file(dest);
                std::os::unix::fs::symlink(source, dest)
            }
            #[cfg(windows)]
            {
                let _ = fs::remove_file(dest);
                if source.is_dir() {
                    std::os::windows::fs::symlink_dir(source, dest)
                } else {
                    std::os::windows::fs::symlink_file(source, dest)
                }
            }
        }
        SyncStrategy::Reflink => match reflink_copy::reflink(source, dest) {
            Ok(()) => Ok(()),
            Err(_) => fs::copy(source, dest).map(|_| ()),
        },
        SyncStrategy::Copy => fs::copy(source, dest).map(|_| ()),
    }?;

    // Preserve source mtime on destination so future checks are instant metadata hits
    if let Ok(meta) = fs::metadata(source) {
        let _ = set_file_mtime(dest, FileTime::from_last_modification_time(&meta));
    }

    Ok(())
}
