mod list_files;
mod target_resolve;

pub use list_files::list_files;
pub use target_resolve::{parse_package_name, resolve_vendor_target};

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let vendor = consumer_root.join("vendor");
    let mut markers = vec![
        vendor.join("composer").join("installed.json"),
        consumer_root.join("composer.lock"),
        vendor.join("composer").join("autoload_classmap.php"),
        vendor.join("composer").join("autoload_static.php"),
    ];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("composer.lock"));
    }
    markers
}

pub fn autoload_hint(source: &Path, consumer: &Path) -> Option<String> {
    let vendor_pkg = consumer.join("vendor");
    if !vendor_pkg.exists() {
        return None;
    }
    let has_php = walkdir::WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|entry| {
            let path = entry.path();
            if path.is_file() {
                let not_excluded = !path.components().any(|c| {
                    matches!(
                        c.as_os_str().to_string_lossy().as_ref(),
                        ".git" | "node_modules" | "vendor" | "target"
                    )
                });
                not_excluded && path.extension().is_some_and(|ext| ext == "php")
            } else {
                false
            }
        });

    if has_php {
        Some("New PHP classes may require: composer dump-autoload (in consumer project)".into())
    } else {
        None
    }
}
