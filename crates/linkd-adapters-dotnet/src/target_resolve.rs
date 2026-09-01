use std::path::Path;

use linkd_core::{IsolationMode, LinkdError, LinkdResult, ResolvedSyncTarget};

pub fn parse_package_name(source: &Path) -> LinkdResult<String> {
    if let Ok(entries) = std::fs::read_dir(source) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext == "csproj" || ext == "fsproj" {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if let Some(start) = trimmed.find("<PackageId>") {
                                if let Some(end) = trimmed.find("</PackageId>") {
                                    let id = &trimmed[start + 11..end].trim();
                                    if !id.is_empty() {
                                        return Ok(id.to_string());
                                    }
                                }
                            }
                        }
                    }
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        return Ok(stem.to_string());
                    }
                }
            }
        }
    }

    let build_props = source.join("Directory.Build.props");
    if build_props.is_file() {
        if let Ok(content) = std::fs::read_to_string(&build_props) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(start) = trimmed.find("<PackageId>") {
                    if let Some(end) = trimmed.find("</PackageId>") {
                        let id = &trimmed[start + 11..end].trim();
                        if !id.is_empty() {
                            return Ok(id.to_string());
                        }
                    }
                }
            }
        }
    }

    source
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
        .ok_or_else(|| LinkdError::PackageNotFound(".NET project (.csproj/.fsproj)".into()))
}

pub fn resolve_dotnet_target(
    consumer_root: &Path,
    package_name: &str,
) -> LinkdResult<ResolvedSyncTarget> {
    let target = consumer_root.join("packages").join(package_name);

    Ok(ResolvedSyncTarget {
        logical_target: target.clone(),
        sync_target: target,
        isolation_mode: IsolationMode::ProjectLocal,
        forbidden_roots: vec![],
    })
}

pub fn detect_dotnet(source: &Path, consumer: &Path) -> bool {
    has_dotnet_manifest(source) || has_dotnet_manifest(consumer)
}

fn has_dotnet_manifest(dir: &Path) -> bool {
    if dir.join("Directory.Build.props").is_file()
        || dir.join("global.json").is_file()
        || dir.join("NuGet.Config").is_file()
    {
        return true;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                if matches!(ext, "csproj" | "fsproj" | "sln") {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_csproj_package_id() {
        let temp = tempfile::tempdir().unwrap();
        let csproj = temp.path().join("MyLib.csproj");
        std::fs::write(
            &csproj,
            r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <PackageId>Acme.Logging.Core</PackageId>
    <Version>1.0.0</Version>
  </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let name = parse_package_name(temp.path()).unwrap();
        assert_eq!(name, "Acme.Logging.Core");
    }

    #[test]
    fn falls_back_to_csproj_filename_stem() {
        let temp = tempfile::tempdir().unwrap();
        let csproj = temp.path().join("CommonUtils.csproj");
        std::fs::write(
            &csproj,
            r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
</Project>"#,
        )
        .unwrap();

        let name = parse_package_name(temp.path()).unwrap();
        assert_eq!(name, "CommonUtils");
    }
}
