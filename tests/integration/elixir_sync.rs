use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn elixir_dep_sync_works() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("elixir_plugin");
    let consumer = tmp.path().join("phoenix_app");

    fs::create_dir_all(source.join("lib")).unwrap();
    fs::write(
        source.join("mix.exs"),
        br#"defmodule ElixirPlugin.MixProject do
  use Mix.Project

  def project do
    [
      app: :elixir_plugin,
      version: "0.1.0"
    ]
  end
end
"#,
    )
    .unwrap();
    fs::write(
        source.join("lib").join("elixir_plugin.ex"),
        b"defmodule ElixirPlugin do\n  def hello, do: :world\nend\n",
    )
    .unwrap();

    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("mix.exs"),
        br#"defmodule PhoenixApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :phoenix_app,
      version: "0.1.0"
    ]
  end
end
"#,
    )
    .unwrap();

    let resolved = resolve_link(&source, &consumer, None, None).unwrap();
    assert_eq!(resolved.ecosystem, Ecosystem::Elixir);
    assert_eq!(resolved.package_name, "elixir_plugin");
    assert_eq!(
        resolved.sync_target,
        consumer.join("deps").join("elixir_plugin")
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
    let ex_file = resolved.sync_target.join("lib").join("elixir_plugin.ex");
    assert!(ex_file.exists(), "synced elixir_plugin.ex must exist");
    assert_eq!(
        fs::read_to_string(ex_file).unwrap(),
        "defmodule ElixirPlugin do\n  def hello, do: :world\nend\n"
    );
}
