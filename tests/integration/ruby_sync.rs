use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn ruby_gem_sync_works() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("ruby-lib");
    let consumer = tmp.path().join("ruby-app");

    fs::create_dir_all(source.join("lib")).unwrap();
    fs::write(
        source.join("acme_auth.gemspec"),
        br#"Gem::Specification.new do |spec|
  spec.name = "acme_auth"
  spec.version = "1.0.0"
end
"#,
    )
    .unwrap();
    fs::write(
        source.join("lib").join("acme_auth.rb"),
        b"module AcmeAuth; def self.version; '1.0.0'; end; end\n",
    )
    .unwrap();

    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("Gemfile"),
        br#"source 'https://rubygems.org'
gem 'acme_auth'
"#,
    )
    .unwrap();

    let resolved = resolve_link(&source, &consumer, None, None).unwrap();
    assert_eq!(resolved.ecosystem, Ecosystem::Ruby);
    assert_eq!(resolved.package_name, "acme_auth");
    assert_eq!(
        resolved.sync_target,
        consumer
            .join("vendor")
            .join("bundle")
            .join("gems")
            .join("acme_auth")
    );

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
    let rb_file = resolved.sync_target.join("lib").join("acme_auth.rb");
    assert!(rb_file.exists(), "synced acme_auth.rb must exist");
    assert_eq!(
        fs::read_to_string(rb_file).unwrap(),
        "module AcmeAuth; def self.version; '1.0.0'; end; end\n"
    );
}
