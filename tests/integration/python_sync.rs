use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn python_pyproject_sync_works() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("py-lib");
    let consumer = tmp.path().join("py-app");

    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("pyproject.toml"),
        br#"[project]
name = "py-lib"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(
        source.join("__init__.py"),
        b"def get_version(): return '0.1.0-synced'\n",
    )
    .unwrap();

    let venv_site = consumer.join(".venv").join("Lib").join("site-packages");
    fs::create_dir_all(&venv_site).unwrap();
    fs::write(
        consumer.join("pyproject.toml"),
        br#"[project]
name = "py-app"
dependencies = ["py-lib"]
"#,
    )
    .unwrap();

    let resolved = resolve_link(&source, &consumer, None, None).unwrap();
    assert_eq!(resolved.ecosystem, Ecosystem::Python);
    assert_eq!(resolved.package_name, "py-lib");
    assert_eq!(resolved.sync_target, venv_site.join("py_lib"));

    let store = RegistryStore::new(tmp.path().join("registry.json"));
    let entry = Registry::new_link(NewLinkParams {
        package_name: resolved.package_name,
        source_path: source.clone(),
        consumer_root: consumer.clone(),
        sync_target: resolved.sync_target.clone(),
        ecosystem: resolved.ecosystem,
        link_mode: LinkMode::PackageManager,
        custom_target: None,
        detected_pm: resolved.detected_pm,
        strategy: SyncStrategy::Copy,
        isolation_mode: IsolationMode::ProjectLocal,
    });
    store.add_link(entry.clone()).unwrap();

    let reconciler = Reconciler::new(RegistryStore::new(store.path().to_path_buf()));
    reconciler.reconcile_link(entry.id).unwrap();

    assert!(LinkMarker::read(&resolved.sync_target).unwrap().is_some());
    let init_py = resolved.sync_target.join("__init__.py");
    assert!(init_py.exists(), "synced __init__.py must exist");
    assert_eq!(
        fs::read_to_string(init_py).unwrap(),
        "def get_version(): return '0.1.0-synced'\n"
    );
}
