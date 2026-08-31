use std::path::{Path, PathBuf};

use linkd_core::{Ecosystem, LinkEntry, LinkMode, LinkdResult, ResolvedSyncTarget};

use crate::{
    CargoAdapter, ComposerAdapter, CustomAdapter, EcosystemAdapter, GoAdapter, JvmAdapter,
    NpmAdapter, PythonAdapter,
};

pub fn adapter_for(ecosystem: Ecosystem) -> Box<dyn EcosystemAdapter> {
    match ecosystem {
        Ecosystem::Npm => Box::new(NpmAdapter),
        Ecosystem::Composer => Box::new(ComposerAdapter),
        Ecosystem::Python => Box::new(PythonAdapter),
        Ecosystem::Go => Box::new(GoAdapter),
        Ecosystem::Cargo => Box::new(CargoAdapter),
        Ecosystem::Jvm => Box::new(JvmAdapter),
        Ecosystem::Custom => Box::new(CustomAdapter),
    }
}

pub fn detect_ecosystem(source: &Path, consumer: &Path) -> Ecosystem {
    if source.join("pyproject.toml").is_file()
        || source.join("setup.py").is_file()
        || source.join("setup.cfg").is_file()
    {
        return Ecosystem::Python;
    }
    if source.join("go.mod").is_file() {
        return Ecosystem::Go;
    }
    if source.join("Cargo.toml").is_file() {
        return Ecosystem::Cargo;
    }
    if source.join("pom.xml").is_file()
        || source.join("build.gradle").is_file()
        || source.join("build.gradle.kts").is_file()
    {
        return Ecosystem::Jvm;
    }
    if source.join("composer.json").is_file() && !source.join("package.json").is_file() {
        return Ecosystem::Composer;
    }
    if source.join("package.json").is_file() {
        return Ecosystem::Npm;
    }

    if consumer.join(".venv").exists()
        || consumer.join("venv").exists()
        || consumer.join("Pipfile").is_file()
    {
        return Ecosystem::Python;
    }
    if consumer.join("go.mod").is_file() || consumer.join("go.work").is_file() {
        return Ecosystem::Go;
    }
    if consumer.join("Cargo.toml").is_file() {
        return Ecosystem::Cargo;
    }
    if consumer.join("pom.xml").is_file() || consumer.join("build.gradle").is_file() {
        return Ecosystem::Jvm;
    }
    if consumer.join("composer.json").is_file() && !consumer.join("package.json").is_file() {
        return Ecosystem::Composer;
    }

    Ecosystem::Npm
}

pub struct ResolvedLink {
    pub package_name: String,
    pub sync_target: PathBuf,
    pub resolved: ResolvedSyncTarget,
    pub ecosystem: Ecosystem,
    pub link_mode: LinkMode,
    pub detected_pm: Option<String>,
}

pub fn resolve_link(
    source: &Path,
    consumer: &Path,
    ecosystem: Option<Ecosystem>,
    custom_target: Option<&Path>,
) -> LinkdResult<ResolvedLink> {
    let eco = if custom_target.is_some() {
        Ecosystem::Custom
    } else {
        ecosystem.unwrap_or_else(|| detect_ecosystem(source, consumer))
    };

    let adapter = adapter_for(eco);
    let package_name = adapter.package_name(source)?;
    let resolved = adapter.resolve_target(consumer, &package_name, custom_target)?;

    crate::validate_link_paths(source, &resolved.sync_target)?;

    let link_mode = if custom_target.is_some() {
        LinkMode::CustomPath
    } else {
        LinkMode::PackageManager
    };

    let detected_pm = match eco {
        Ecosystem::Npm => Some(
            format!("{:?}", linkd_adapters_npm::detect_package_manager(consumer)).to_lowercase(),
        ),
        Ecosystem::Composer => Some("composer".into()),
        Ecosystem::Python => Some("uv/pip".into()),
        Ecosystem::Go => Some("go".into()),
        Ecosystem::Cargo => Some("cargo".into()),
        Ecosystem::Jvm => Some("maven/gradle".into()),
        Ecosystem::Custom => None,
    };

    Ok(ResolvedLink {
        package_name,
        sync_target: resolved.sync_target.clone(),
        resolved,
        ecosystem: eco,
        link_mode,
        detected_pm,
    })
}

pub fn completion_markers_for_link(link: &LinkEntry) -> Vec<PathBuf> {
    let adapter = adapter_for(link.ecosystem);
    match link.link_mode {
        LinkMode::CustomPath => {
            let mut paths = vec![link.source_path.clone(), link.sync_target.clone()];
            paths.sort();
            paths.dedup();
            paths
        }
        LinkMode::PackageManager => adapter.completion_markers(&link.consumer_root),
    }
}

pub fn list_files_for_link(link: &LinkEntry) -> LinkdResult<Vec<PathBuf>> {
    let adapter = adapter_for(link.ecosystem);
    adapter.list_files(&link.source_path)
}

pub fn post_sync_hint_for_link(link: &LinkEntry) -> Option<String> {
    let adapter = adapter_for(link.ecosystem);
    adapter.post_sync_hint(&link.source_path, &link.consumer_root)
}

pub fn resolve_for_reconcile(link: &LinkEntry) -> LinkdResult<ResolvedSyncTarget> {
    let adapter = adapter_for(link.ecosystem);
    let custom = link.custom_target.as_deref();
    adapter.resolve_target(&link.consumer_root, &link.package_name, custom)
}

pub fn build_allowlist_for_link(
    link: &LinkEntry,
    resolved: &ResolvedSyncTarget,
) -> linkd_sync::WriteAllowlist {
    match link.ecosystem {
        Ecosystem::Npm => linkd_adapters_npm::PnpmStoreDetector::build_allowlist(
            &link.consumer_root,
            resolved.forbidden_roots.clone(),
        ),
        Ecosystem::Composer => linkd_sync::WriteAllowlist::from_consumer_subdirs(
            &link.consumer_root,
            &["vendor"],
            resolved.forbidden_roots.clone(),
        ),
        Ecosystem::Python => linkd_sync::WriteAllowlist::from_consumer_subdirs(
            &link.consumer_root,
            &[".venv", "venv", "env"],
            resolved.forbidden_roots.clone(),
        ),
        Ecosystem::Go => linkd_sync::WriteAllowlist::from_consumer_subdirs(
            &link.consumer_root,
            &["vendor"],
            resolved.forbidden_roots.clone(),
        ),
        Ecosystem::Cargo => linkd_sync::WriteAllowlist::from_consumer_subdirs(
            &link.consumer_root,
            &["vendor", ".cargo"],
            resolved.forbidden_roots.clone(),
        ),
        Ecosystem::Jvm => linkd_sync::WriteAllowlist::from_consumer_subdirs(
            &link.consumer_root,
            &["libs"],
            resolved.forbidden_roots.clone(),
        ),
        Ecosystem::Custom => {
            let target = resolved.sync_target.clone();
            let allowed = target
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| link.consumer_root.clone());
            linkd_sync::WriteAllowlist::from_allowed_roots(
                vec![allowed],
                resolved.forbidden_roots.clone(),
            )
        }
    }
}

pub fn ensure_isolation(link: &LinkEntry, resolved: &ResolvedSyncTarget) -> LinkdResult<()> {
    if link.ecosystem == Ecosystem::Npm {
        crate::ensure_shadow_isolation(&link.consumer_root, &link.package_name, resolved)?;
    }
    if link.ecosystem == Ecosystem::Npm {
        crate::assert_never_writes_global_store(&resolved.sync_target)?;
    }
    Ok(())
}
