use std::path::Path;

use crate::commands::build::env_toml::{self, ENV_FILE};
use crate::commands::check_engine_constraint;
use crate::commands::doctor::diagnosis::{Category, Check, Context, Diagnosis};
use crate::commands::{EngineConstraintError, version};
use crate::config::{self, CONFIG_FILE};
use crate::extension::{ExtensionListStatus, list as list_extensions};

/// Where the schema-version fixes point for migration instructions.
const MIGRATION_URL: &str = "https://github.com/stellar-scaffold/cli/blob/main/CHANGELOG.md";

/// Validates `scaffold.yml`: the full config for version 2, otherwise whether
/// the project falls back to `environments.toml`, and the default directories.
pub struct ScaffoldYml;

#[async_trait::async_trait]
impl Check for ScaffoldYml {
    fn name(&self) -> &'static str {
        "scaffold-yml"
    }

    fn category(&self) -> Category {
        Category::Project
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        let Some(root) = ctx.workspace_root else {
            return vec![Diagnosis::skipped(
                self.name(),
                Category::Project,
                "not in a Cargo workspace",
            )];
        };

        if let Some(loaded) = &ctx.config {
            return config_diagnoses(self.name(), root, loaded);
        }
        // Version 2 files are loaded and reported above, so passing here means
        // the project is configured by environments.toml.

        // Matched per variant rather than reusing the error text, which is too
        // long for one report line. The fix carries the migration link instead.
        let current = config::SCHEMA_VERSION;
        let version = match config::check_version(root) {
            Ok(()) => Diagnosis::warn(
                self.name(),
                Category::Project,
                format!("configured by the deprecated {ENV_FILE}"),
            )
            .with_fix(format!(
                "migrate to schema version {current}; see {MIGRATION_URL}"
            )),
            Err(config::VersionError::MissingVersion) => Diagnosis::error(
                self.name(),
                Category::Project,
                format!("{CONFIG_FILE} is missing or has no 'version' field"),
            )
            .with_fix(format!(
                "add 'version: {current}' to {CONFIG_FILE}; see {MIGRATION_URL}"
            )),
            Err(config::VersionError::UnsupportedVersion { found }) => Diagnosis::error(
                self.name(),
                Category::Project,
                format!("{CONFIG_FILE} uses schema version {found}, this CLI supports {current}"),
            )
            .with_fix(format!("see {MIGRATION_URL}")),
        };

        let project = config::project(root);
        vec![
            version,
            dir_exists(
                "contracts-dir",
                root,
                &project.contracts_dir,
                "create it, or migrate to schema version 2 and set project.contracts-dir",
            ),
            dir_exists(
                "clients-dir",
                root,
                &project.clients_dir,
                "stellar scaffold build --build-clients",
            ),
        ]
    }
}

/// Findings for a version 2 `scaffold.yml`: one per config diagnostic, named
/// by its code, then the directories its `project:` section points at.
fn config_diagnoses(name: &'static str, root: &Path, loaded: &config::Loaded) -> Vec<Diagnosis> {
    let errors = loaded.diagnostics.iter().filter(|d| d.is_error()).count();
    let summary = if errors == 0 {
        Diagnosis::ok(
            name,
            Category::Project,
            format!("{CONFIG_FILE} schema version {}", config::SCHEMA_VERSION),
        )
    } else {
        Diagnosis::error(
            name,
            Category::Project,
            format!(
                "{CONFIG_FILE} has {errors} problem{}",
                if errors == 1 { "" } else { "s" }
            ),
        )
        .with_fix("stellar scaffold config check")
    };
    let mut findings = vec![summary];
    for d in &loaded.diagnostics {
        let message = format!("{}: {}", d.location(CONFIG_FILE), d.message);
        let finding = match d.severity {
            config::Severity::Error => Diagnosis::error(d.code.slug(), Category::Project, message),
            config::Severity::Warning => Diagnosis::warn(d.code.slug(), Category::Project, message),
        };
        findings.push(match &d.help {
            Some(help) => finding.with_fix(help.clone()),
            None => finding,
        });
    }
    if let Some(config) = &loaded.config {
        findings.push(dir_exists(
            "contracts-dir",
            root,
            &config.project.contracts_dir,
            "create it, or point project.contracts-dir at the right path",
        ));
        findings.push(dir_exists(
            "clients-dir",
            root,
            &config.project.clients_dir,
            "stellar scaffold build --build-clients",
        ));
    }
    findings
}

/// Warns when a directory `scaffold.yml` points at is absent. Not an error: a
/// fresh project has no generated clients yet.
fn dir_exists(name: &'static str, root: &Path, dir: &Path, fix: &str) -> Diagnosis {
    let display = dir.display();
    if root.join(dir).is_dir() {
        Diagnosis::ok(name, Category::Project, format!("{display}"))
    } else {
        Diagnosis::warn(name, Category::Project, format!("{display} does not exist")).with_fix(fix)
    }
}

/// Checks the running CLI against the project's `engines.stellar-scaffold`
/// constraint in `package.json`.
pub struct EngineConstraint;

#[async_trait::async_trait]
impl Check for EngineConstraint {
    fn name(&self) -> &'static str {
        "engine-constraint"
    }

    fn category(&self) -> Category {
        Category::Project
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        let Some(root) = ctx.workspace_root else {
            return vec![Diagnosis::skipped(
                self.name(),
                Category::Project,
                "not in a Cargo workspace",
            )];
        };

        // An absent package.json or constraint is Ok, not Skipped: a project
        // that pins nothing is satisfied by any version.
        let diagnosis = match check_engine_constraint(root) {
            Ok(()) => Diagnosis::ok(
                self.name(),
                Category::Project,
                format!("stellar-scaffold {}", version::pkg()),
            ),
            Err(EngineConstraintError::ConstraintNotSatisfied {
                required,
                installed,
            }) => Diagnosis::error(
                self.name(),
                Category::Project,
                format!("project requires stellar-scaffold {required}, found {installed}"),
            )
            .with_fix("cargo install stellar-scaffold-cli"),
            Err(e @ EngineConstraintError::InvalidConstraint { .. }) => {
                Diagnosis::error(self.name(), Category::Project, e.to_string())
            }
        };

        vec![diagnosis]
    }
}

/// Warns when `.env` is absent but `.env.example` is there to copy from.
pub struct DotEnv;

#[async_trait::async_trait]
impl Check for DotEnv {
    fn name(&self) -> &'static str {
        "dot-env"
    }

    fn category(&self) -> Category {
        Category::Project
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        let Some(root) = ctx.workspace_root else {
            return vec![skipped_no_workspace(self.name())];
        };

        let diagnosis = if root.join(".env").exists() {
            Diagnosis::ok(self.name(), Category::Project, ".env present")
        } else if root.join(".env.example").exists() {
            Diagnosis::warn(self.name(), Category::Project, ".env is missing")
                .with_fix("cp .env.example .env")
        } else {
            // Neither file: nothing to copy from, and not every project uses one.
            Diagnosis::skipped(self.name(), Category::Project, "no .env or .env.example")
        };

        vec![diagnosis]
    }
}

/// Validates `environments.toml`: that it parses, defines the selected
/// environment, has a usable network, and names real contracts.
pub struct EnvironmentsToml;

#[async_trait::async_trait]
impl Check for EnvironmentsToml {
    fn name(&self) -> &'static str {
        "environments-toml"
    }

    fn category(&self) -> Category {
        Category::Project
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        if ctx.workspace_root.is_none() {
            return vec![skipped_no_workspace(self.name())];
        }

        let env = match &ctx.environment {
            Err(e) => {
                return vec![Diagnosis::error(
                    self.name(),
                    Category::Project,
                    e.to_string(),
                )];
            }
            // A contracts-only project can legitimately have no file at all.
            Ok(None) => {
                return vec![Diagnosis::skipped(
                    self.name(),
                    Category::Project,
                    format!("no {ENV_FILE}"),
                )];
            }
            Ok(Some(env)) => env,
        };

        let scaffold_env = ctx.env;
        let mut findings = vec![Diagnosis::ok(
            self.name(),
            Category::Project,
            format!("{scaffold_env:?} environment configured"),
        )];

        findings.push(network_diagnosis(&env.network));
        findings.extend(contract_diagnoses(env, &ctx.package_names));
        findings
    }
}

/// A network needs either a named network or an explicit url plus passphrase.
fn network_diagnosis(network: &env_toml::Network) -> Diagnosis {
    const NAME: &str = "network-config";

    if let Some(name) = &network.name {
        return Diagnosis::ok(NAME, Category::Project, format!("network {name}"));
    }
    if network.rpc_url.is_some() && network.network_passphrase.is_some() {
        return Diagnosis::ok(NAME, Category::Project, "network url and passphrase set");
    }
    Diagnosis::error(
        NAME,
        Category::Project,
        "network needs a name, or both rpc-url and network-passphrase",
    )
    .with_fix(format!("set network.name in {ENV_FILE}"))
}

/// Checks that each contract key names a package in the workspace. Contracts
/// pinned to a deployed `id` are skipped — they have no local source.
fn contract_diagnoses(env: &env_toml::Environment, package_names: &[String]) -> Vec<Diagnosis> {
    const NAME: &str = "contract-names";

    let Some(contracts) = env.contracts.as_ref() else {
        return Vec::new();
    };

    // Package names reach the build as wasm filenames, where hyphens become
    // underscores, so compare both sides in the normalized form.
    let known: Vec<String> = package_names.iter().map(|n| normalize(n)).collect();
    let unknown: Vec<String> = contracts
        .iter()
        .filter(|(_, settings)| settings.id.is_none())
        .map(|(name, _)| name.to_string())
        .filter(|name| !known.contains(&normalize(name)))
        .collect();

    if unknown.is_empty() {
        return vec![Diagnosis::ok(
            NAME,
            Category::Project,
            format!("{} contract(s) resolved", contracts.len()),
        )];
    }

    vec![
        Diagnosis::error(
            NAME,
            Category::Project,
            format!("no package in the workspace named: {}", unknown.join(", ")),
        )
        .with_fix(format!(
            "fix the contract keys in {ENV_FILE}, or add the crate"
        )),
    ]
}

fn normalize(name: &str) -> String {
    name.replace('-', "_")
}

/// Reports extensions the current environment lists but cannot run.
pub struct Extensions;

#[async_trait::async_trait]
impl Check for Extensions {
    fn name(&self) -> &'static str {
        "extensions"
    }

    fn category(&self) -> Category {
        Category::Project
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        if ctx.workspace_root.is_none() {
            return vec![skipped_no_workspace(self.name())];
        }

        let Some(env) = ctx.environment() else {
            return vec![Diagnosis::skipped(
                self.name(),
                Category::Project,
                format!("no {ENV_FILE}"),
            )];
        };

        if env.extensions.is_empty() {
            return vec![Diagnosis::skipped(
                self.name(),
                Category::Project,
                "no extensions configured",
            )];
        }

        list_extensions(&env.extensions)
            .into_iter()
            .map(|entry| {
                let name = entry.name;
                match entry.status {
                    ExtensionListStatus::Found { version, .. } => {
                        Diagnosis::ok("extension", Category::Project, format!("{name} {version}"))
                    }
                    // Warn, not error: a missing extension only breaks the hooks
                    // that use it, and builds still run without them.
                    ExtensionListStatus::MissingBinary => Diagnosis::warn(
                        "extension",
                        Category::Project,
                        format!("{name} binary not found on PATH"),
                    )
                    .with_fix(format!("install stellar-scaffold-{name}")),
                    ExtensionListStatus::ManifestError(e) => Diagnosis::error(
                        "extension",
                        Category::Project,
                        format!("{name} manifest failed: {e}"),
                    ),
                }
            })
            .collect()
    }
}

fn skipped_no_workspace(name: &'static str) -> Diagnosis {
    Diagnosis::skipped(name, Category::Project, "not in a Cargo workspace")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::build::clients::ScaffoldEnv;
    use crate::commands::doctor::diagnosis::{NO_COMMANDS, Severity};
    use stellar_cli::print::Print;

    fn context<'a>(workspace_root: Option<&'a Path>, printer: &'a Print) -> Context<'a> {
        Context {
            workspace_root,
            commands: &*NO_COMMANDS,
            env: ScaffoldEnv::Development,
            environment: Ok(None),
            package_names: Vec::new(),
            config: None,
            printer,
        }
    }

    /// A context whose environment is parsed from `environments.toml` in `root`.
    fn context_with_env<'a>(root: &'a Path, packages: &[&str], printer: &'a Print) -> Context<'a> {
        Context {
            workspace_root: Some(root),
            commands: &*NO_COMMANDS,
            env: ScaffoldEnv::Development,
            environment: env_toml::Environment::get(root, &ScaffoldEnv::Development),
            package_names: packages.iter().map(ToString::to_string).collect(),
            config: None,
            printer,
        }
    }

    fn write_env_toml(root: &Path, contents: &str) {
        std::fs::write(root.join(ENV_FILE), contents).unwrap();
    }

    fn severity_of(findings: &[Diagnosis], name: &str) -> Severity {
        findings
            .iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("no diagnosis named {name}"))
            .severity
    }

    #[tokio::test]
    async fn scaffold_yml_skipped_outside_workspace() {
        let printer = Print::new(true);
        let findings = ScaffoldYml.run(&context(None, &printer)).await;
        assert_eq!(severity_of(&findings, "scaffold-yml"), Severity::Skipped);
    }

    #[tokio::test]
    async fn scaffold_yml_warns_on_environments_toml() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(ENV_FILE), "").unwrap();
        std::fs::create_dir_all(dir.path().join("contracts")).unwrap();
        std::fs::create_dir_all(dir.path().join("app-lib/clients")).unwrap();

        let printer = Print::new(true);
        let findings = ScaffoldYml.run(&context(Some(dir.path()), &printer)).await;

        assert_eq!(severity_of(&findings, "scaffold-yml"), Severity::Warn);
        assert_eq!(severity_of(&findings, "contracts-dir"), Severity::Ok);
        assert_eq!(severity_of(&findings, "clients-dir"), Severity::Ok);
    }

    #[tokio::test]
    async fn scaffold_yml_checks_version_1_dirs_beside_environments_toml() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(ENV_FILE), "").unwrap();
        std::fs::write(
            dir.path().join(CONFIG_FILE),
            "version: 1\nconfig:\n  contracts_dir: rust\n",
        )
        .unwrap();
        // Only the custom directory exists, not the default `contracts/`.
        std::fs::create_dir_all(dir.path().join("rust")).unwrap();

        let printer = Print::new(true);
        let findings = ScaffoldYml.run(&context(Some(dir.path()), &printer)).await;

        assert_eq!(severity_of(&findings, "contracts-dir"), Severity::Ok);
    }

    /// A context carrying a version 2 `scaffold.yml` loaded from `root`.
    fn context_with_config<'a>(root: &'a Path, yaml: &str, printer: &'a Print) -> Context<'a> {
        std::fs::write(root.join(CONFIG_FILE), yaml).unwrap();
        Context {
            config: Some(config::load(root, Some(&[]), None)),
            ..context(Some(root), printer)
        }
    }

    #[tokio::test]
    async fn scaffold_yml_v2_ok_reports_project_dirs() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src/contracts")).unwrap();
        let yaml = "version: 2\nproject:\n  contracts-dir: src/contracts\nnetworks:\n  local: {}\n";
        let printer = Print::new(true);
        let findings = ScaffoldYml
            .run(&context_with_config(dir.path(), yaml, &printer))
            .await;

        assert_eq!(severity_of(&findings, "scaffold-yml"), Severity::Ok);
        assert_eq!(severity_of(&findings, "contracts-dir"), Severity::Ok);
        assert_eq!(severity_of(&findings, "clients-dir"), Severity::Warn);
    }

    #[tokio::test]
    async fn scaffold_yml_v2_reports_each_problem_by_code() {
        let dir = tempfile::TempDir::new().unwrap();
        let yaml = "version: 2\nnetworks:\n  preview: { extends: testnet }\n";
        let printer = Print::new(true);
        let findings = ScaffoldYml
            .run(&context_with_config(dir.path(), yaml, &printer))
            .await;

        assert_eq!(severity_of(&findings, "scaffold-yml"), Severity::Error);
        let problem = findings
            .iter()
            .find(|d| d.name == "unknown-network")
            .unwrap();
        assert!(
            problem.message.starts_with("scaffold.yml:3:23: "),
            "{}",
            problem.message
        );
        assert_eq!(
            problem.fix.as_deref(),
            Some("add `testnet: {}` under `networks:`")
        );
    }

    #[tokio::test]
    async fn scaffold_yml_errors_when_absent_and_warns_on_missing_dirs() {
        let dir = tempfile::TempDir::new().unwrap();
        let printer = Print::new(true);
        let findings = ScaffoldYml.run(&context(Some(dir.path()), &printer)).await;

        assert_eq!(severity_of(&findings, "scaffold-yml"), Severity::Error);
        assert_eq!(severity_of(&findings, "contracts-dir"), Severity::Warn);
        assert_eq!(severity_of(&findings, "clients-dir"), Severity::Warn);
    }

    #[tokio::test]
    async fn scaffold_yml_errors_on_unsupported_version() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(CONFIG_FILE), "version: 99\n").unwrap();

        let printer = Print::new(true);
        let findings = ScaffoldYml.run(&context(Some(dir.path()), &printer)).await;

        let version = findings.iter().find(|d| d.name == "scaffold-yml").unwrap();
        assert_eq!(version.severity, Severity::Error);
        assert!(version.message.contains("99"), "got: {}", version.message);
    }

    #[tokio::test]
    async fn engine_constraint_ok_without_package_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let printer = Print::new(true);
        let findings = EngineConstraint
            .run(&context(Some(dir.path()), &printer))
            .await;
        assert_eq!(severity_of(&findings, "engine-constraint"), Severity::Ok);
    }

    #[tokio::test]
    async fn engine_constraint_errors_when_unsatisfied() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"engines": {"stellar-scaffold": ">=999.0.0"}}"#,
        )
        .unwrap();

        let printer = Print::new(true);
        let findings = EngineConstraint
            .run(&context(Some(dir.path()), &printer))
            .await;

        let finding = findings.first().unwrap();
        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(
            finding.fix.as_deref(),
            Some("cargo install stellar-scaffold-cli")
        );
    }

    #[tokio::test]
    async fn dot_env_warns_when_only_example_present() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env.example"), "").unwrap();

        let printer = Print::new(true);
        let findings = DotEnv.run(&context(Some(dir.path()), &printer)).await;

        let finding = findings.first().unwrap();
        assert_eq!(finding.severity, Severity::Warn);
        assert_eq!(finding.fix.as_deref(), Some("cp .env.example .env"));
    }

    #[tokio::test]
    async fn dot_env_ok_when_present() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(".env"), "").unwrap();

        let printer = Print::new(true);
        let findings = DotEnv.run(&context(Some(dir.path()), &printer)).await;
        assert_eq!(severity_of(&findings, "dot-env"), Severity::Ok);
    }

    #[tokio::test]
    async fn dot_env_skipped_when_neither_file_exists() {
        let dir = tempfile::TempDir::new().unwrap();
        let printer = Print::new(true);
        let findings = DotEnv.run(&context(Some(dir.path()), &printer)).await;
        assert_eq!(severity_of(&findings, "dot-env"), Severity::Skipped);
    }

    #[tokio::test]
    async fn environments_toml_skipped_when_absent() {
        let dir = tempfile::TempDir::new().unwrap();
        let printer = Print::new(true);
        let findings = EnvironmentsToml
            .run(&context_with_env(dir.path(), &[], &printer))
            .await;
        assert_eq!(
            severity_of(&findings, "environments-toml"),
            Severity::Skipped
        );
    }

    #[tokio::test]
    async fn environments_toml_errors_when_malformed() {
        let dir = tempfile::TempDir::new().unwrap();
        write_env_toml(dir.path(), "this is not valid toml {{{");

        let printer = Print::new(true);
        let findings = EnvironmentsToml
            .run(&context_with_env(dir.path(), &[], &printer))
            .await;
        assert_eq!(severity_of(&findings, "environments-toml"), Severity::Error);
    }

    #[tokio::test]
    async fn environments_toml_ok_with_known_contract() {
        let dir = tempfile::TempDir::new().unwrap();
        write_env_toml(
            dir.path(),
            r#"
development.network = { name = "local" }
development.contracts.hello_world.client = true
"#,
        );

        let printer = Print::new(true);
        let findings = EnvironmentsToml
            .run(&context_with_env(dir.path(), &["hello-world"], &printer))
            .await;

        assert_eq!(severity_of(&findings, "environments-toml"), Severity::Ok);
        assert_eq!(severity_of(&findings, "network-config"), Severity::Ok);
        // Hyphens in the package name normalize to the underscored contract key.
        assert_eq!(severity_of(&findings, "contract-names"), Severity::Ok);
    }

    #[tokio::test]
    async fn environments_toml_errors_on_unknown_contract() {
        let dir = tempfile::TempDir::new().unwrap();
        write_env_toml(
            dir.path(),
            r#"
development.network = { name = "local" }
development.contracts.typoed_name.client = true
"#,
        );

        let printer = Print::new(true);
        let findings = EnvironmentsToml
            .run(&context_with_env(dir.path(), &["hello_world"], &printer))
            .await;

        let finding = findings
            .iter()
            .find(|d| d.name == "contract-names")
            .unwrap();
        assert_eq!(finding.severity, Severity::Error);
        assert!(
            finding.message.contains("typoed_name"),
            "got: {}",
            finding.message
        );
    }

    #[tokio::test]
    async fn environments_toml_skips_contracts_pinned_to_an_id() {
        let dir = tempfile::TempDir::new().unwrap();
        write_env_toml(
            dir.path(),
            r#"
development.network = { name = "local" }
development.contracts.remote.id = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
"#,
        );

        let printer = Print::new(true);
        let findings = EnvironmentsToml
            .run(&context_with_env(dir.path(), &[], &printer))
            .await;
        assert_eq!(severity_of(&findings, "contract-names"), Severity::Ok);
    }

    #[tokio::test]
    async fn environments_toml_errors_on_incomplete_network() {
        let dir = tempfile::TempDir::new().unwrap();
        write_env_toml(
            dir.path(),
            r#"
development.network = { rpc-url = "http://localhost:8000/rpc" }
"#,
        );

        let printer = Print::new(true);
        let findings = EnvironmentsToml
            .run(&context_with_env(dir.path(), &[], &printer))
            .await;
        assert_eq!(severity_of(&findings, "network-config"), Severity::Error);
    }

    #[tokio::test]
    async fn extensions_skipped_when_none_configured() {
        let dir = tempfile::TempDir::new().unwrap();
        write_env_toml(dir.path(), "development.network = { name = \"local\" }\n");

        let printer = Print::new(true);
        let findings = Extensions
            .run(&context_with_env(dir.path(), &[], &printer))
            .await;
        assert_eq!(severity_of(&findings, "extensions"), Severity::Skipped);
    }

    #[tokio::test]
    async fn extensions_warns_on_missing_binary() {
        let dir = tempfile::TempDir::new().unwrap();
        write_env_toml(
            dir.path(),
            r#"
development.network = { name = "local" }
development.extensions = ["definitely-not-installed"]
"#,
        );

        let printer = Print::new(true);
        let findings = Extensions
            .run(&context_with_env(dir.path(), &[], &printer))
            .await;

        let finding = findings.first().unwrap();
        assert_eq!(finding.severity, Severity::Warn);
        assert!(
            finding.message.contains("definitely-not-installed"),
            "got: {}",
            finding.message
        );
    }
}
