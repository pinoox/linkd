mod detect;
mod list_files;
mod target_resolve;

pub use detect::{detect_jvm, parse_jvm_package_name};
pub use list_files::list_files;
pub use target_resolve::resolve_jvm_target;

use std::path::{Path, PathBuf};

pub fn completion_markers(consumer_root: &Path) -> Vec<PathBuf> {
    let mut markers = vec![
        consumer_root.join("pom.xml"),
        consumer_root.join("build.gradle"),
        consumer_root.join("build.gradle.kts"),
        consumer_root.join("gradle.lockfile"),
        consumer_root.join("gradlew"),
    ];
    markers.retain(|p| p.parent().map(|parent| parent.exists()).unwrap_or(false));
    if markers.is_empty() {
        markers.push(consumer_root.join("pom.xml"));
    }
    markers
}

pub fn post_sync_hint(source: &Path, _consumer: &Path) -> Option<String> {
    if source.join("pom.xml").exists()
        || source.join("build.gradle").exists()
        || source.join("build.gradle.kts").exists()
    {
        Some("JVM source files synced. Run Gradle/Maven with `--refresh-dependencies` or restart language server.".into())
    } else {
        None
    }
}
