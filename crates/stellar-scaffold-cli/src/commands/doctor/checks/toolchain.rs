use std::path::Path;

use crate::commands::PackageManagerSpec;
use crate::commands::doctor::diagnosis::{Category, Check, CommandOutput, Context, Diagnosis};

/// Contract build target. Every scaffold project compiles to this.
const WASM_TARGET: &str = "wasm32v1-none";

/// The `stellar-cli` this build was compiled against (see build.rs).
const PINNED_STELLAR_CLI_VERSION: &str = env!("PINNED_STELLAR_CLI_VERSION");

/// Checks `rustc` is installed, and matches `rust-toolchain.toml` if present.
pub struct Rustc;

#[async_trait::async_trait]
impl Check for Rustc {
    fn name(&self) -> &'static str {
        "rustc"
    }

    fn category(&self) -> Category {
        Category::Toolchain
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        let Some(output) = ctx
            .commands
            .run("rustc", &["--version"])
            .stdout()
            .map(str::to_string)
        else {
            return vec![
                Diagnosis::error(self.name(), Category::Toolchain, "rustc not found")
                    .with_fix("install Rust from https://rustup.rs"),
            ];
        };

        // "rustc 1.93.0 (254b59607 2026-01-19)" -> "1.93.0"
        let Some(installed) = output.split_whitespace().nth(1) else {
            return vec![Diagnosis::warn(
                self.name(),
                Category::Toolchain,
                format!("could not read a version from: {output}"),
            )];
        };

        let Some(required) = ctx.workspace_root.and_then(toolchain_channel) else {
            return vec![Diagnosis::ok(self.name(), Category::Toolchain, installed)];
        };

        // The channel can be a name like "stable" rather than a version, in
        // which case there is nothing to compare against.
        if required == installed || !required.starts_with(|c: char| c.is_ascii_digit()) {
            Diagnosis::ok(
                self.name(),
                Category::Toolchain,
                format!("{installed} (rust-toolchain.toml: {required})"),
            )
        } else {
            Diagnosis::warn(
                self.name(),
                Category::Toolchain,
                format!("rust-toolchain.toml pins {required}, found {installed}"),
            )
            .with_fix(format!("rustup toolchain install {required}"))
        }
        .into_vec()
    }
}

/// Reads the `toolchain.channel` value from a project's `rust-toolchain.toml`.
fn toolchain_channel(root: &Path) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Toolchain {
        channel: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct File {
        toolchain: Option<Toolchain>,
    }

    let contents = std::fs::read_to_string(root.join("rust-toolchain.toml")).ok()?;
    toml::from_str::<File>(&contents).ok()?.toolchain?.channel
}

/// Checks the wasm target contracts compile to is installed.
pub struct WasmTarget;

#[async_trait::async_trait]
impl Check for WasmTarget {
    fn name(&self) -> &'static str {
        "wasm-target"
    }

    fn category(&self) -> Category {
        Category::Toolchain
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        let diagnosis = match ctx
            .commands
            .run("rustup", &["target", "list", "--installed"])
        {
            CommandOutput::Success(targets) if targets.lines().any(|t| t.trim() == WASM_TARGET) => {
                Diagnosis::ok(self.name(), Category::Toolchain, WASM_TARGET)
            }
            CommandOutput::Success(_) => Diagnosis::error(
                self.name(),
                Category::Toolchain,
                format!("{WASM_TARGET} is not installed"),
            )
            .with_fix(format!("rustup target add {WASM_TARGET}")),
            // A Nix- or distro-managed toolchain has no rustup, and may still
            // have the target — this check just can't confirm it.
            CommandOutput::NotFound => Diagnosis::skipped(
                self.name(),
                Category::Toolchain,
                "rustup not found; cannot list targets",
            ),
            CommandOutput::Failed => Diagnosis::warn(
                self.name(),
                Category::Toolchain,
                "rustup could not list installed targets",
            ),
        };

        vec![diagnosis]
    }
}

/// Checks the `stellar` CLI is installed and its major matches this build.
pub struct StellarCli;

#[async_trait::async_trait]
impl Check for StellarCli {
    fn name(&self) -> &'static str {
        "stellar-cli"
    }

    fn category(&self) -> Category {
        Category::Toolchain
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        let Some(output) = ctx
            .commands
            .run("stellar", &["--version"])
            .stdout()
            .map(str::to_string)
        else {
            return vec![
                Diagnosis::error(self.name(), Category::Toolchain, "stellar not found")
                    .with_fix("cargo install --locked stellar-cli"),
            ];
        };

        // "stellar 27.0.0 (5a7c5fe...)" on the first line.
        let installed = output
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1));
        let Some(installed) = installed else {
            return vec![Diagnosis::warn(
                self.name(),
                Category::Toolchain,
                format!("could not read a version from: {output}"),
            )];
        };

        // Majors track the protocol version, so a mismatch there is what breaks
        // builds; patch differences are fine.
        let pinned_major = major(PINNED_STELLAR_CLI_VERSION);
        if major(installed) == pinned_major {
            Diagnosis::ok(self.name(), Category::Toolchain, installed)
        } else {
            Diagnosis::warn(
                self.name(),
                Category::Toolchain,
                format!(
                    "found {installed}, this CLI was built against {PINNED_STELLAR_CLI_VERSION}"
                ),
            )
            .with_fix("cargo install --locked stellar-cli")
        }
        .into_vec()
    }
}

/// Leading numeric component of a version or version requirement.
fn major(version: &str) -> Option<u64> {
    version
        .trim_start_matches(['=', '^', '~', '>', '<', 'v'])
        .split('.')
        .next()?
        .parse()
        .ok()
}

/// Checks Node and the project's package manager are installed.
pub struct NodeToolchain;

#[async_trait::async_trait]
impl Check for NodeToolchain {
    fn name(&self) -> &'static str {
        "node"
    }

    fn category(&self) -> Category {
        Category::Toolchain
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        // A contracts-only project has no package.json and needs no JS tooling.
        let Some(spec) = ctx
            .workspace_root
            .and_then(PackageManagerSpec::from_package_json)
        else {
            return vec![Diagnosis::skipped(
                self.name(),
                Category::Toolchain,
                "no package manager configured",
            )];
        };

        let node_output = ctx.commands.run("node", &["--version"]);
        let node = match node_output.stdout() {
            Some(v) => Diagnosis::ok(self.name(), Category::Toolchain, v),
            None => Diagnosis::error(self.name(), Category::Toolchain, "node not found")
                .with_fix("install Node.js from https://nodejs.org"),
        };

        vec![node, package_manager_diagnosis(ctx, &spec)]
    }
}

/// Compares the installed package manager against the `packageManager` field.
fn package_manager_diagnosis(ctx: &Context<'_>, spec: &PackageManagerSpec) -> Diagnosis {
    const NAME: &str = "package-manager";

    let command = spec.kind.command();
    let output = ctx.commands.run(command, &["--version"]);
    let Some(installed) = output.stdout() else {
        return Diagnosis::error(NAME, Category::Toolchain, format!("{command} not found"))
            .with_fix(format!("corepack enable {command}"));
    };

    match &spec.version {
        Some(pinned) if pinned != installed => Diagnosis::warn(
            NAME,
            Category::Toolchain,
            format!("package.json pins {command}@{pinned}, found {installed}"),
        )
        .with_fix(format!("corepack use {command}@{pinned}")),
        _ => Diagnosis::ok(NAME, Category::Toolchain, format!("{command} {installed}")),
    }
}

/// Wraps a single diagnosis, so the common case reads without a `vec![]`.
trait IntoVec {
    fn into_vec(self) -> Vec<Diagnosis>;
}

impl IntoVec for Diagnosis {
    fn into_vec(self) -> Vec<Diagnosis> {
        vec![self]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::build::clients::ScaffoldEnv;
    use crate::commands::doctor::diagnosis::{CommandRunner, FakeCommands, Severity};
    use stellar_cli::print::Print;

    fn context<'a>(
        commands: &'a dyn CommandRunner,
        workspace_root: Option<&'a Path>,
        printer: &'a Print,
    ) -> Context<'a> {
        Context {
            workspace_root,
            commands,
            env: ScaffoldEnv::Development,
            environment: Ok(None),
            package_names: Vec::new(),
            printer,
        }
    }

    fn only(findings: Vec<Diagnosis>) -> Diagnosis {
        assert_eq!(findings.len(), 1, "expected one diagnosis");
        findings.into_iter().next().unwrap()
    }

    fn named(findings: &[Diagnosis], name: &str) -> Diagnosis {
        findings
            .iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("no diagnosis named {name}"))
            .clone()
    }

    #[tokio::test]
    async fn rustc_errors_when_missing() {
        let commands = FakeCommands::new();
        let printer = Print::new(true);
        let finding = only(Rustc.run(&context(&commands, None, &printer)).await);

        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(
            finding.fix.as_deref(),
            Some("install Rust from https://rustup.rs")
        );
    }

    #[tokio::test]
    async fn rustc_ok_without_a_toolchain_file() {
        let commands =
            FakeCommands::new().succeeding("rustc", "rustc 1.93.0 (254b59607 2026-01-19)");
        let printer = Print::new(true);
        let finding = only(Rustc.run(&context(&commands, None, &printer)).await);

        assert_eq!(finding.severity, Severity::Ok);
        assert_eq!(finding.message, "1.93.0");
    }

    #[tokio::test]
    async fn rustc_warns_on_channel_mismatch() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"1.99.0\"\n",
        )
        .unwrap();

        let commands =
            FakeCommands::new().succeeding("rustc", "rustc 1.93.0 (254b59607 2026-01-19)");
        let printer = Print::new(true);
        let finding = only(
            Rustc
                .run(&context(&commands, Some(dir.path()), &printer))
                .await,
        );

        assert_eq!(finding.severity, Severity::Warn);
        assert_eq!(
            finding.fix.as_deref(),
            Some("rustup toolchain install 1.99.0")
        );
    }

    #[tokio::test]
    async fn rustc_ok_for_a_named_channel() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"stable\"\n",
        )
        .unwrap();

        let commands =
            FakeCommands::new().succeeding("rustc", "rustc 1.93.0 (254b59607 2026-01-19)");
        let printer = Print::new(true);
        let finding = only(
            Rustc
                .run(&context(&commands, Some(dir.path()), &printer))
                .await,
        );

        // "stable" names no version, so there is nothing to mismatch against.
        assert_eq!(finding.severity, Severity::Ok);
    }

    #[tokio::test]
    async fn wasm_target_ok_when_installed() {
        let commands =
            FakeCommands::new().succeeding("rustup", "aarch64-apple-darwin\nwasm32v1-none");
        let printer = Print::new(true);
        let finding = only(WasmTarget.run(&context(&commands, None, &printer)).await);
        assert_eq!(finding.severity, Severity::Ok);
    }

    #[tokio::test]
    async fn wasm_target_errors_when_absent() {
        let commands = FakeCommands::new().succeeding("rustup", "aarch64-apple-darwin");
        let printer = Print::new(true);
        let finding = only(WasmTarget.run(&context(&commands, None, &printer)).await);

        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(
            finding.fix.as_deref(),
            Some("rustup target add wasm32v1-none")
        );
    }

    #[tokio::test]
    async fn wasm_target_skipped_without_rustup() {
        let commands = FakeCommands::new();
        let printer = Print::new(true);
        let finding = only(WasmTarget.run(&context(&commands, None, &printer)).await);

        // A Nix toolchain has no rustup but may still have the target.
        assert_eq!(finding.severity, Severity::Skipped);
    }

    #[tokio::test]
    async fn stellar_cli_ok_on_matching_major() {
        let version = format!("stellar {PINNED_STELLAR_CLI_VERSION} (abc123)");
        let commands = FakeCommands::new().succeeding("stellar", &version);
        let printer = Print::new(true);
        let finding = only(StellarCli.run(&context(&commands, None, &printer)).await);
        assert_eq!(finding.severity, Severity::Ok);
    }

    #[tokio::test]
    async fn stellar_cli_warns_on_major_mismatch() {
        let commands = FakeCommands::new().succeeding("stellar", "stellar 1.0.0 (abc123)");
        let printer = Print::new(true);
        let finding = only(StellarCli.run(&context(&commands, None, &printer)).await);
        assert_eq!(finding.severity, Severity::Warn);
    }

    #[tokio::test]
    async fn stellar_cli_errors_when_missing() {
        let commands = FakeCommands::new();
        let printer = Print::new(true);
        let finding = only(StellarCli.run(&context(&commands, None, &printer)).await);
        assert_eq!(finding.severity, Severity::Error);
    }

    #[tokio::test]
    async fn node_skipped_without_package_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let commands = FakeCommands::new();
        let printer = Print::new(true);
        let finding = only(
            NodeToolchain
                .run(&context(&commands, Some(dir.path()), &printer))
                .await,
        );
        assert_eq!(finding.severity, Severity::Skipped);
    }

    #[tokio::test]
    async fn node_warns_when_package_manager_version_differs() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"packageManager": "pnpm@9.6.0"}"#,
        )
        .unwrap();

        let commands = FakeCommands::new()
            .succeeding("node", "v24.16.0")
            .succeeding("pnpm", "8.15.1");
        let printer = Print::new(true);
        let findings = NodeToolchain
            .run(&context(&commands, Some(dir.path()), &printer))
            .await;

        assert_eq!(named(&findings, "node").severity, Severity::Ok);
        let pm = named(&findings, "package-manager");
        assert_eq!(pm.severity, Severity::Warn);
        assert_eq!(pm.fix.as_deref(), Some("corepack use pnpm@9.6.0"));
    }

    #[tokio::test]
    async fn node_errors_when_package_manager_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"packageManager": "pnpm@9.6.0"}"#,
        )
        .unwrap();

        let commands = FakeCommands::new().succeeding("node", "v24.16.0");
        let printer = Print::new(true);
        let findings = NodeToolchain
            .run(&context(&commands, Some(dir.path()), &printer))
            .await;

        assert_eq!(
            named(&findings, "package-manager").severity,
            Severity::Error
        );
    }
}
