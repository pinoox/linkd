//! Tests syncing both JS and PHP packages in a multi-ecosystem monorepo setup.

use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, LinkMarker, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn monorepo_side_by_side_js_and_php_sync() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // 1. Create monorepo layout
    let js_pkg = root.join("packages").join("js-lib");
    let php_pkg = root.join("packages").join("php-lib");
    let app = root.join("apps").join("web");

    fs::create_dir_all(&js_pkg).unwrap();
    fs::write(
        js_pkg.join("package.json"),
        br#"{"name":"@monorepo/js-lib","version":"1.0.0","main":"index.js"}"#,
    )
    .unwrap();
    fs::write(
        js_pkg.join("index.js"),
        b"module.exports = { env: 'dev-js' };\n",
    )
    .unwrap();

    fs::create_dir_all(php_pkg.join("src")).unwrap();
    fs::write(
        php_pkg.join("composer.json"),
        br#"{"name":"acme/php-lib","autoload":{"psr-4":{"Acme\\":"src/"}}}"#,
    )
    .unwrap();
    fs::write(
        php_pkg.join("src").join("Helper.php"),
        b"<?php namespace Acme; class Helper { public static function env() { return 'dev-php'; } }",
    )
    .unwrap();

    fs::create_dir_all(app.join("node_modules").join("@monorepo").join("js-lib")).unwrap();
    fs::create_dir_all(app.join("vendor").join("acme").join("php-lib").join("src")).unwrap();
    fs::write(
        app.join("node_modules")
            .join("@monorepo")
            .join("js-lib")
            .join("index.js"),
        b"module.exports = { env: 'stale-js' };\n",
    )
    .unwrap();
    fs::write(
        app.join("vendor").join("acme").join("php-lib").join("src").join("Helper.php"),
        b"<?php namespace Acme; class Helper { public static function env() { return 'stale-php'; } }",
    )
    .unwrap();

    let store = RegistryStore::new(root.join("registry.json"));

    // 2. Resolve & Register JS link
    let resolved_js = resolve_link(&js_pkg, &app, Some(Ecosystem::Npm), None).unwrap();
    let entry_js = Registry::new_link(NewLinkParams {
        package_name: resolved_js.package_name.clone(),
        source_path: js_pkg.clone(),
        consumer_root: app.clone(),
        sync_target: resolved_js.sync_target.clone(),
        ecosystem: resolved_js.ecosystem,
        link_mode: resolved_js.link_mode,
        custom_target: None,
        detected_pm: resolved_js.detected_pm,
        strategy: SyncStrategy::Copy,
        isolation_mode: resolved_js.resolved.isolation_mode,
    });
    store.add_link(entry_js.clone()).unwrap();

    // 3. Resolve & Register PHP link
    let resolved_php = resolve_link(&php_pkg, &app, Some(Ecosystem::Composer), None).unwrap();
    let entry_php = Registry::new_link(NewLinkParams {
        package_name: resolved_php.package_name.clone(),
        source_path: php_pkg.clone(),
        consumer_root: app.clone(),
        sync_target: resolved_php.sync_target.clone(),
        ecosystem: resolved_php.ecosystem,
        link_mode: resolved_php.link_mode,
        custom_target: None,
        detected_pm: resolved_php.detected_pm,
        strategy: SyncStrategy::Copy,
        isolation_mode: resolved_php.resolved.isolation_mode,
    });
    store.add_link(entry_php.clone()).unwrap();

    // 4. Reconcile both
    let reconciler = Reconciler::new(RegistryStore::new(store.path().to_path_buf()));
    reconciler.reconcile_all().unwrap();

    // 5. Verify JS synced
    let js_target = resolved_js.sync_target;
    assert!(LinkMarker::read(&js_target).unwrap().is_some());
    let js_content = fs::read_to_string(js_target.join("index.js")).unwrap();
    assert!(js_content.contains("dev-js"));

    // 6. Verify PHP synced
    let php_target = resolved_php.sync_target;
    assert!(LinkMarker::read(&php_target).unwrap().is_some());
    let php_content = fs::read_to_string(php_target.join("src").join("Helper.php")).unwrap();
    assert!(php_content.contains("dev-php"));
}
