//! The `instantiate` step of `init`: pure-filesystem assembly of selected
//! framework template and package manager into a runnable project.
//!
//! After `acquire` (degit) drops the UI monorepo into the project path,
//! `instantiate` selects one framework, promotes it to `app/`, removes the
//! other templates, and rewrites the root workspaces. No network, no prompts;
//! all choices are passed in as parameters for easy offline unit-testing.

use std::fs;
use std::path::Path;

use crate::commands::{PackageManager, PackageManagerSpec};

/// The directory inside the UI monorepo that holds per-framework templates
pub const TEMPLATES_DIR: &str = "templates";
/// The directory selected template is promoted to
pub const APP_DIR: &str = "app";

/// The declared workspaces for instantiated project, handled differently
/// depending on the selected package manager
const INSTANTIATED_WORKSPACES: &[&str] = &["app", "bindings/*", "core"];

const DENO_CONFIG: &str = "{\n  \"nodeModulesDir\": \"auto\"\n}\n";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(
        "unknown framework {name:?}; available templates: {}",
        if available.is_empty() { "none found".to_string() } else { available.join(", ") }
    )]
    UnknownFramework {
        name: String,
        available: Vec<String>,
    },
    #[error("no templates/ directory found — is this the UI monorepo?")]
    NoTemplatesDir,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed root package.json: {0}")]
    PackageJson(#[from] serde_json::Error),
}

/// How `--template` (or the interactive prompt) selected a source.
///
/// Slash heuristic: a bare token names an official framework resolved
/// against directories within `templates/`; a token containing a `/`
/// is a community repo shorthand (`org/repo`) degit'd directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
    /// Official framework within the UI monorepo, e.g. `"react"`.
    Framework(String),
    /// Community repo shorthand, e.g. `"org/repo#ref"`.
    Community(String),
}

/// Parse a `--template` value into official or community source
pub fn parse_template_arg(value: &str) -> TemplateSource {
    if value.contains('/') {
        TemplateSource::Community(value.to_string())
    } else {
        TemplateSource::Framework(value.to_string())
    }
}

/// List framework names available under `<root>/templates/`, sorted.
pub fn enumerate_templates(root: &Path) -> Result<Vec<String>, Error> {
    let dir = root.join(TEMPLATES_DIR);
    if !dir.is_dir() {
        return Err(Error::NoTemplatesDir);
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// Promote the selected framework template to `app/`, drop the rest, and
/// rewrite the root workspaces.
pub fn instantiate(root: &Path, framework: &str) -> Result<(), Error> {
    let templates = root.join(TEMPLATES_DIR);
    let selected = templates.join(framework);
    if !selected.is_dir() {
        return Err(Error::UnknownFramework {
            name: framework.to_string(),
            available: enumerate_templates(root).unwrap_or_default(),
        });
    }

    // Move selected out, then remove the remaining templates/ dir. The
    // absence of templates/ afterward is what marks a correctly-initialized
    // project versus a raw clone of the monorepo.
    let app = root.join(APP_DIR);
    fs::rename(&selected, &app)?;
    fs::remove_dir_all(&templates)?;

    rewrite_root_workspaces(root)?;
    Ok(())
}

/// Rewrite the `workspaces` field of the root `package.json` to the
/// instantiated set. Note this might reserialize the JSON, but that's
/// fine on a newly scaffolded project as long as `git init` happens
/// in later step so `git status` is clean at completion.
fn rewrite_root_workspaces(root: &Path) -> Result<(), Error> {
    let path = root.join("package.json");
    let contents = fs::read_to_string(&path)?;
    let mut value: serde_json::Value = serde_json::from_str(&contents)?;
    value["workspaces"] = serde_json::json!(INSTANTIATED_WORKSPACES);
    let mut out = serde_json::to_string_pretty(&value)?;
    out.push('\n');
    fs::write(&path, out)?;
    Ok(())
}

/// pnpm-workspace.yaml contents mirroring the instantiated workspaces. pnpm
/// reads workspace globs from this file rather than `package.json`.
fn pnpm_workspace_yaml() -> String {
    let mut out = String::from("packages:\n");
    for ws in INSTANTIATED_WORKSPACES {
        out.push_str("  - \"");
        out.push_str(ws);
        out.push_str("\"\n");
    }
    out
}

/// Apply the selected package manager to the instantiated project: write the
/// `packageManager` field, emit any manager-specific workspace/config file, and
/// drop the npm lockfile when switching away from npm. Pure filesystem work.
pub fn apply_package_manager(root: &Path, spec: &PackageManagerSpec) -> Result<(), Error> {
    spec.write_to_package_json(root)?;

    match spec.kind {
        PackageManager::Pnpm => {
            fs::write(root.join("pnpm-workspace.yaml"), pnpm_workspace_yaml())?;
        }
        PackageManager::Deno => {
            fs::write(root.join("deno.json"), DENO_CONFIG)?;
        }
        _ => {}
    }

    if spec.kind != PackageManager::Npm {
        let lock = root.join("package-lock.json");
        if lock.exists() {
            fs::remove_file(lock)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    /// Build a minimal post-degit monorepo fixture in a temp dir.
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            &root.join("package.json"),
            "{\n  \"name\": \"ui\",\n  \"workspaces\": [\"templates/*\", \"bindings/*\", \"core\"]\n}\n",
        );
        write(&root.join("templates/react/index.html"), "<react/>");
        write(&root.join("templates/svelte/index.html"), "<svelte/>");
        write(
            &root.join("core/package.json"),
            "{\"name\":\"@stellar-scaffold/ui-core\"}",
        );
        write(&root.join("contracts/.gitkeep"), "");
        dir
    }

    #[test]
    fn parse_template_arg_framework_when_no_slash() {
        assert_eq!(
            parse_template_arg("react"),
            TemplateSource::Framework("react".into())
        );
    }

    #[test]
    fn parse_template_arg_community_when_slash() {
        assert_eq!(
            parse_template_arg("org/repo#tutorial"),
            TemplateSource::Community("org/repo#tutorial".into())
        );
    }

    #[test]
    fn enumerate_templates_lists_sorted_frameworks() {
        let dir = fixture();
        assert_eq!(
            enumerate_templates(dir.path()).unwrap(),
            vec!["react".to_string(), "svelte".to_string()]
        );
    }

    #[test]
    fn enumerate_templates_errors_without_templates_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            enumerate_templates(dir.path()),
            Err(Error::NoTemplatesDir)
        ));
    }

    #[test]
    fn instantiate_promotes_one_and_drops_rest() {
        let dir = fixture();
        let root = dir.path();
        instantiate(root, "react").unwrap();

        assert!(root.join("app/index.html").exists(), "app/ created");
        assert_eq!(
            fs::read_to_string(root.join("app/index.html")).unwrap(),
            "<react/>"
        );
        assert!(!root.join("templates").exists(), "templates/ removed");
        assert!(root.join("core/package.json").exists(), "core/ untouched");
    }

    #[test]
    fn instantiate_rewrites_root_workspaces() {
        let dir = fixture();
        let root = dir.path();
        instantiate(root, "svelte").unwrap();

        let pkg: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
        assert_eq!(
            pkg["workspaces"],
            serde_json::json!(["app", "bindings/*", "core"])
        );
    }

    #[test]
    fn instantiate_unknown_framework_errors_with_available() {
        let dir = fixture();
        let err = instantiate(dir.path(), "jquery-lol").unwrap_err();
        match err {
            Error::UnknownFramework { name, available } => {
                assert_eq!(name, "jquery-lol");
                assert_eq!(available, vec!["react".to_string(), "svelte".to_string()]);
            }
            other => panic!("expected UnknownFramework, got {other:?}"),
        }
        // failure leaves the monorepo untouched
        assert!(dir.path().join("templates/react").exists());
    }

    fn spec(kind: PackageManager) -> PackageManagerSpec {
        PackageManagerSpec {
            kind,
            version: Some("1.2.3".into()),
        }
    }

    #[test]
    fn apply_pnpm_writes_workspace_yaml_and_field() {
        let dir = fixture();
        let root = dir.path();
        apply_package_manager(root, &spec(PackageManager::Pnpm)).unwrap();

        let yaml = fs::read_to_string(root.join("pnpm-workspace.yaml")).unwrap();
        assert!(yaml.contains("- \"app\""));
        assert!(yaml.contains("- \"bindings/*\""));
        assert!(yaml.contains("- \"core\""));

        let pkg = fs::read_to_string(root.join("package.json")).unwrap();
        assert!(pkg.contains("\"packageManager\": \"pnpm@1.2.3\""));
    }

    #[test]
    fn apply_deno_writes_deno_json() {
        let dir = fixture();
        let root = dir.path();
        apply_package_manager(root, &spec(PackageManager::Deno)).unwrap();
        assert!(root.join("deno.json").exists());
        assert!(!root.join("pnpm-workspace.yaml").exists());
    }

    #[test]
    fn apply_non_npm_removes_lockfile() {
        let dir = fixture();
        let root = dir.path();
        write(&root.join("package-lock.json"), "{}");
        apply_package_manager(root, &spec(PackageManager::Bun)).unwrap();
        assert!(!root.join("package-lock.json").exists());
    }

    #[test]
    fn apply_npm_keeps_lockfile_and_writes_no_extra_config() {
        let dir = fixture();
        let root = dir.path();
        write(&root.join("package-lock.json"), "{}");
        apply_package_manager(root, &spec(PackageManager::Npm)).unwrap();
        assert!(root.join("package-lock.json").exists());
        assert!(!root.join("pnpm-workspace.yaml").exists());
        assert!(!root.join("deno.json").exists());
    }
}
