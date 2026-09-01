use std::path::Path;

use linkd_core::{IsolationMode, LinkdError, LinkdResult, ResolvedSyncTarget};

pub fn parse_package_name(source: &Path) -> LinkdResult<String> {
    let manifest = source.join("mix.exs");
    if !manifest.is_file() {
        return Err(LinkdError::PackageNotFound("mix.exs not found".into()));
    }
    let content = std::fs::read_to_string(&manifest).map_err(|e| LinkdError::io(&manifest, e))?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(pos) = trimmed.find("app:") {
            let after = &trimmed[pos + 4..].trim();
            if let Some(atom) = after.strip_prefix(':') {
                let name: String = atom
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    return Ok(name);
                }
            } else if let Some(stripped) = after.strip_prefix('"') {
                let name: String = stripped.chars().take_while(|c| *c != '"').collect();
                if !name.is_empty() {
                    return Ok(name);
                }
            }
        }
    }

    source
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
        .ok_or_else(|| LinkdError::PackageNotFound("Elixir app name in mix.exs".into()))
}

pub fn resolve_elixir_target(
    consumer_root: &Path,
    package_name: &str,
) -> LinkdResult<ResolvedSyncTarget> {
    let target = consumer_root.join("deps").join(package_name);

    Ok(ResolvedSyncTarget {
        logical_target: target.clone(),
        sync_target: target,
        isolation_mode: IsolationMode::ProjectLocal,
        forbidden_roots: vec![],
    })
}

pub fn detect_elixir(source: &Path, consumer: &Path) -> bool {
    source.join("mix.exs").is_file()
        || consumer.join("mix.exs").is_file()
        || consumer.join("mix.lock").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mix_exs_atom_app() {
        let temp = tempfile::tempdir().unwrap();
        let mix = temp.path().join("mix.exs");
        std::fs::write(
            &mix,
            r#"defmodule MyElixirPkg.MixProject do
  use Mix.Project

  def project do
    [
      app: :phoenix_live_tools,
      version: "0.1.0",
      elixir: "~> 1.14",
      deps: deps()
    ]
  end
end"#,
        )
        .unwrap();

        let name = parse_package_name(temp.path()).unwrap();
        assert_eq!(name, "phoenix_live_tools");
    }
}
