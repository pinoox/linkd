use std::fs;

use linkd_adapters::resolve_link;
use linkd_core::{Ecosystem, IsolationMode, LinkMarker, LinkMode, SyncStrategy};
use linkd_daemon::Reconciler;
use linkd_registry::{NewLinkParams, Registry, RegistryStore};
use tempfile::TempDir;

#[test]
fn dotnet_package_sync_works() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("DotnetLib");
    let consumer = tmp.path().join("DotnetApp");

    fs::create_dir_all(source.join("src")).unwrap();
    fs::write(
        source.join("DotnetLib.csproj"),
        br#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <PackageId>Acme.Core.Logging</PackageId>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
</Project>
"#,
    )
    .unwrap();
    fs::write(
        source.join("src").join("Logger.cs"),
        b"namespace Acme.Core.Logging { public class Logger {} }\n",
    )
    .unwrap();

    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("DotnetApp.csproj"),
        br#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
</Project>
"#,
    )
    .unwrap();

    let resolved = resolve_link(&source, &consumer, None, None).unwrap();
    assert_eq!(resolved.ecosystem, Ecosystem::Dotnet);
    assert_eq!(resolved.package_name, "Acme.Core.Logging");
    assert_eq!(
        resolved.sync_target,
        consumer.join("packages").join("Acme.Core.Logging")
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
    let cs_file = resolved.sync_target.join("src").join("Logger.cs");
    assert!(cs_file.exists(), "synced Logger.cs must exist");
    assert_eq!(
        fs::read_to_string(cs_file).unwrap(),
        "namespace Acme.Core.Logging { public class Logger {} }\n"
    );
}
