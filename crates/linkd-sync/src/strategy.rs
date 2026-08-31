use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use linkd_core::SyncStrategy;

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
        SyncStrategy::Reflink => match reflink::reflink(source, dest) {
            Ok(()) => Ok(()),
            Err(_) => fs::copy(source, dest).map(|_| ()),
        },
        SyncStrategy::Copy => fs::copy(source, dest).map(|_| ()),
    }
}

pub fn mirror_tree(
    source_root: &Path,
    dest_root: &Path,
    files: &[PathBuf],
    strategy: SyncStrategy,
) -> io::Result<u32> {
    fs::create_dir_all(dest_root)?;
    let mut count = 0u32;

    for rel in files {
        let src = source_root.join(rel);
        let dst = dest_root.join(rel);
        if src.is_dir() {
            fs::create_dir_all(&dst)?;
            continue;
        }
        if src.is_file() {
            copy_file_with_strategy(&src, &dst, strategy)?;
            count += 1;
        }
    }

    Ok(count)
}
