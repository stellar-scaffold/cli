use clap::Parser;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Select};
use std::fs::copy;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::build::env_toml;
use super::init::instantiate::APP_DIR;
use super::{
    EngineConstraintError, PackageManager, PackageManagerSpec, build, check_engine_constraint,
};
use crate::extension::{ExtensionListStatus, list as list_extensions};
use stellar_cli::{commands::global, print::Print};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error(transparent)]
    BuildError(Box<build::Error>),
    #[error(transparent)]
    SchemaVersion(#[from] crate::config::VersionError),
    #[error(transparent)]
    EngineConstraint(#[from] EngineConstraintError),
}

/// The `prepare` step of `init`: the network/tooling tail run after the
/// project's files are in place. Package-manager *selection* and *file writes*
/// happen earlier (in `init` and `instantiate`); this takes the already-chosen
/// manager and does environment setup, dependency install, contract build, and
/// git init. `pkg_manager` is `None` for no-frontend projects, which have no
/// JS dependencies to install.
pub async fn prepare(
    project_path: &Path,
    pkg_manager: Option<&PackageManagerSpec>,
    global_args: &global::Args,
    yes: bool,
) -> Result<(), Error> {
    let printer = Print::new(global_args.quiet);

    crate::config::check_version(project_path)?;
    check_engine_constraint(project_path)?;

    for dir in dot_env_dirs(project_path) {
        if let Err(e) = copy_env_example(&dir) {
            printer.warnln(format!("Failed to copy .env.example: {e}"));
        }
    }

    if let Ok(Some(dev_env)) =
        env_toml::Environment::get(project_path, &build::clients::ScaffoldEnv::Development)
        && dev_env.network.run_locally
        && Command::new("docker").arg("--version").output().is_err()
    {
        printer.warnln("Docker not found. Install it from https://docs.docker.com/get-docker/");
        printer.warnln("Docker is required to run a local Stellar network (run-locally = true in environments.toml).");
    }

    ensure_extensions_installed(project_path, &printer, yes);

    if let Some(pkg_manager) = pkg_manager {
        // A frontend without a JS manifest has no dependencies to install.
        if has_js_manifest(project_path) {
            run_install(pkg_manager.kind.command(), project_path, &printer);
        }
        printer.infoln("Compiling contracts and generating client packages...");
    } else {
        printer.infoln("Compiling contracts...");
    }
    let mut build_command = build::Command::parse_from(["build"]);
    build_command.build.manifest_path = Some(project_path.join("Cargo.toml"));
    // The clients pipeline needs a network (account funding, deploys) even when
    // every contract opts out with `client = false`; a no-frontend project has
    // no clients to generate, so skip it and just compile.
    build_command.build_clients = pkg_manager.is_some();
    build_command.compile_only = pkg_manager.is_none();
    let mut build_args = global_args.clone();
    if !(global_args.verbose && global_args.very_verbose) {
        build_args.quiet = true;
    }
    if let Err(e) = build_command.run(&build_args).await.map_err(Box::new) {
        printer.warnln(format!("Failed to compile contracts: {e}"));
    }

    if git_exists() {
        git_init(project_path);
        if git_has_changes(project_path) {
            git_add(project_path, &["-A"]);
            git_commit(project_path, "initial commit");
        }
    }

    Ok(())
}

/// Directories that may hold a project's `.env`, relative to its root: the
/// root itself, and `app/`, where `init` puts the frontend. Its dev server
/// and the `stellar` CLI it starts both read `app/.env`.
pub(crate) const DOT_ENV_DIRS: [&str; 2] = ["", APP_DIR];

/// The [`DOT_ENV_DIRS`] of the project at `root`.
pub(crate) fn dot_env_dirs(root: &Path) -> impl Iterator<Item = PathBuf> {
    DOT_ENV_DIRS.iter().map(|dir| root.join(dir))
}

/// Copy `.env.example` in `dir` to `.env`, unless there's no example or
/// `.env` already exists.
fn copy_env_example(dir: &Path) -> io::Result<()> {
    let env_path = dir.join(".env");
    let example_path = dir.join(".env.example");
    if !env_path.exists() && example_path.exists() {
        copy(example_path, env_path)?;
    }
    Ok(())
}

/// Resolve a package-manager choice: honor an explicit `-p`, else prompt
/// (or pick the default when `yes`). Returns `None` only if the interactive
/// prompt is cancelled.
pub(crate) fn resolve_pkg_manager(
    requested: Option<&PackageManager>,
    printer: &Print,
    yes: bool,
) -> Option<PackageManagerSpec> {
    match requested {
        Some(kind) => Some(PackageManagerSpec {
            kind: kind.clone(),
            version: pkg_manager_version(kind.command()),
        }),
        None => select_pkg_manager(printer, yes),
    }
}

fn detect_pkg_managers() -> Vec<PackageManagerSpec> {
    PackageManager::LIST
        .iter()
        .filter_map(|kind| {
            let version = pkg_manager_version(kind.command())?;
            Some(PackageManagerSpec {
                kind: kind.clone(),
                version: Some(version),
            })
        })
        .collect()
}

fn select_pkg_manager(printer: &Print, yes: bool) -> Option<PackageManagerSpec> {
    let detected = detect_pkg_managers();

    if detected.is_empty() {
        printer.warnln("No supported package manager detected (npm, pnpm, yarn, bun, deno).");
        printer.warnln("Defaulting to npm — install it from https://nodejs.org");
        return Some(PackageManagerSpec {
            kind: PackageManager::Npm,
            version: None,
        });
    }

    let default_index = detected
        .iter()
        .position(|s| s.kind == PackageManager::Npm)
        .unwrap_or(0);

    if yes || detected.len() == 1 {
        let spec = detected.into_iter().nth(default_index).unwrap();
        let label = format_pm_label(&spec);
        printer.infoln(format!("Using {label}"));
        return Some(spec);
    }

    let labels: Vec<String> = detected.iter().map(format_pm_label).collect();

    let index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a package manager")
        .items(&labels)
        .default(default_index)
        .interact()
        .ok()?;

    detected.into_iter().nth(index)
}

fn format_pm_label(spec: &PackageManagerSpec) -> String {
    match &spec.version {
        Some(v) => format!("{} ({})", spec.kind.as_str(), v),
        None => spec.kind.as_str().to_string(),
    }
}

fn pkg_manager_version(command: &str) -> Option<String> {
    let output = Command::new(command).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    extract_version(&stdout)
}

fn run_install(pm_command: &str, path: &Path, printer: &Print) {
    printer.infoln("Installing dependencies...");
    match Command::new(pm_command)
        .arg("install")
        .current_dir(path)
        .output()
    {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            printer.warnln(format!(
                "Failed to install dependencies: Please run '{pm_command} install' manually"
            ));
            if !output.stderr.is_empty()
                && let Ok(stderr) = String::from_utf8(output.stderr)
            {
                printer.warnln(format!("Error: {}", stderr.trim()));
            }
        }
        Err(e) => {
            printer.warnln(format!("Failed to run {pm_command} install: {e}"));
        }
    }
}

const KNOWN_EXTENSIONS: &[(&str, &str)] =
    &[("reporter", "cargo install stellar-scaffold-reporter")];

fn ensure_extensions_installed(project_path: &Path, printer: &Print, yes: bool) {
    let env_toml_path = project_path.join("environments.toml");
    let Ok(toml_str) = std::fs::read_to_string(&env_toml_path) else {
        return;
    };

    let Ok(mut envs) = toml::from_str::<
        std::collections::HashMap<String, build::env_toml::Environment>,
    >(&toml_str) else {
        printer.warnln("Could not parse environments.toml to check extensions");
        return;
    };

    let mut seen = std::collections::HashSet::new();
    let unique_entries: Vec<_> = envs
        .values_mut()
        .flat_map(|env| env.extensions.drain(..))
        .filter(|e| seen.insert(e.name.clone()))
        .collect();

    if unique_entries.is_empty() {
        return;
    }

    let missing: Vec<String> = list_extensions(&unique_entries)
        .into_iter()
        .filter(|e| matches!(e.status, ExtensionListStatus::MissingBinary))
        .map(|e| e.name)
        .collect();

    if missing.is_empty() {
        return;
    }

    let (known, unknown): (Vec<_>, Vec<_>) = missing
        .iter()
        .partition(|name| KNOWN_EXTENSIONS.iter().any(|(k, _)| k == name));

    if !unknown.is_empty() {
        printer.warnln(format!(
            "Missing 3rd party extensions: {}",
            unknown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        printer.warnln("Install manually and ensure 'stellar-scaffold-<name>' is on your PATH.");
    }

    if known.is_empty() {
        return;
    }

    printer.infoln(format!(
        "Missing official extensions: {}",
        known
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));

    let install = yes
        || Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Install missing extensions?")
            .default(true)
            .interact()
            .unwrap_or(false);

    if !install {
        printer.warnln(
            "Skipped extension installation. Some features may not work until extensions are installed.",
        );
        return;
    }

    for name in &known {
        let install_cmd = KNOWN_EXTENSIONS
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, cmd)| *cmd)
            .unwrap();

        printer.infoln(format!("Running: {install_cmd}"));
        let mut parts = install_cmd.split_whitespace();
        let bin = parts.next().unwrap();
        let args: Vec<_> = parts.collect();
        match Command::new(bin).args(&args).output() {
            Ok(output) if output.status.success() => {
                printer.checkln(format!("'{name}' installed"));
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                printer.warnln(format!("Failed to install '{name}': {}", stderr.trim()));
            }
            Err(e) => {
                printer.warnln(format!("Failed to run '{install_cmd}': {e}"));
            }
        }
    }
}

/// Whether `root` has a manifest a JS package manager installs from.
fn has_js_manifest(root: &Path) -> bool {
    ["package.json", "deno.json", "deno.jsonc"]
        .iter()
        .any(|m| root.join(m).is_file())
}

fn git_exists() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

fn git_init(path: &Path) {
    let _ = Command::new("git").arg("init").current_dir(path).output();
}

fn git_has_changes(path: &Path) -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

fn git_add(path: &Path, rest: &[&str]) {
    let mut args = vec!["add"];
    args.extend_from_slice(rest);
    let _ = Command::new("git").args(args).current_dir(path).output();
}

fn git_commit(path: &Path, message: &str) {
    let _ = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(path)
        .output();
}

fn extract_version(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        if is_semver_like(token) {
            return Some(
                token
                    .trim_matches(|c: char| !c.is_ascii_digit() && c != '.')
                    .to_string(),
            );
        }
    }
    None
}

fn is_semver_like(s: &str) -> bool {
    let s = s.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
    let mut parts = s.split('.');

    let major = parts.next().and_then(|p| p.parse::<u64>().ok());
    let minor = parts.next().and_then(|p| p.parse::<u64>().ok());
    let patch = parts.next().map_or(Some(0), |p| p.parse::<u64>().ok());

    major.is_some() && minor.is_some() && patch.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_example_is_copied_in_the_app_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let app = dir.path().join(APP_DIR);
        std::fs::create_dir(&app).unwrap();
        std::fs::write(app.join(".env.example"), "STELLAR_NETWORK=local\n").unwrap();

        for dir in dot_env_dirs(dir.path()) {
            copy_env_example(&dir).unwrap();
        }

        let env = std::fs::read_to_string(app.join(".env")).unwrap();
        assert_eq!(env, "STELLAR_NETWORK=local\n");
        assert!(!dir.path().join(".env").exists());
    }

    #[test]
    fn existing_env_is_kept() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env.example"), "example").unwrap();
        std::fs::write(dir.path().join(".env"), "mine").unwrap();

        copy_env_example(dir.path()).unwrap();

        let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
        assert_eq!(env, "mine");
    }

    #[test]
    fn extract_version_from_npm_output() {
        assert_eq!(extract_version("10.2.3"), Some("10.2.3".to_string()));
    }

    #[test]
    fn extract_version_from_pnpm_output() {
        assert_eq!(extract_version("9.6.0"), Some("9.6.0".to_string()));
    }

    #[test]
    fn extract_version_from_yarn_output() {
        assert_eq!(extract_version("1.22.19"), Some("1.22.19".to_string()));
    }

    #[test]
    fn extract_version_from_prefixed_string() {
        assert_eq!(extract_version("v1.2.3"), Some("1.2.3".to_string()));
    }

    #[test]
    fn extract_version_ignores_non_version_tokens() {
        assert_eq!(extract_version("npm 10.2.3"), Some("10.2.3".to_string()));
    }

    #[test]
    fn extract_version_returns_none_for_garbage() {
        assert_eq!(extract_version("not-a-version"), None);
    }

    #[test]
    fn is_semver_like_two_part_accepted() {
        assert!(is_semver_like("1.22"));
    }

    #[test]
    fn is_semver_like_three_part() {
        assert!(is_semver_like("10.2.3"));
    }

    #[test]
    fn is_semver_like_rejects_word() {
        assert!(!is_semver_like("npm"));
    }

    #[test]
    fn is_semver_like_rejects_single_number() {
        assert!(!is_semver_like("10"));
    }
}
