#![allow(clippy::struct_excessive_bools)]
use crate::commands::build::Error::EmptyPackageName;
use crate::commands::version;
use crate::extension;
use cargo_metadata::camino::Utf8PathBuf;
use cargo_metadata::{Metadata, MetadataCommand, Package};
use clap::Parser;
use clients::ScaffoldEnv;
use serde_json::Value;
use std::collections::BTreeMap;
use std::{fmt::Debug, io, path::Path, process::ExitStatus};
use stellar_cli::commands::contract::build::Cmd;
use stellar_cli::commands::{contract::build, global};
use stellar_cli::print::Print;
use stellar_scaffold_ext_types::{CompileContext, HookName};

pub mod clients;
pub mod docker;
pub mod env_toml;
pub mod v2;

/// Build a contract from source
///
/// Builds all crates that are referenced by the cargo manifest (Cargo.toml)
/// that have cdylib as their crate-type. Crates are built for the wasm32
/// target. Unless configured otherwise, crates are built with their default
/// features and with their release profile.
///
/// To view the commands that will be executed, without executing them, use the
/// --print-commands-only option.
#[derive(Parser, Debug, Clone)]
pub struct Command {
    /// List package names in order of build
    #[arg(long, visible_alias = "ls")]
    pub list: bool,
    #[command(flatten)]
    pub build: build::Cmd,
    /// Build client code in addition to building the contract
    #[arg(long)]
    pub build_clients: bool,
    #[command(flatten)]
    pub build_clients_args: clients::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Metadata(#[from] cargo_metadata::Error),
    #[error(transparent)]
    EnvironmentsToml(#[from] env_toml::Error),
    #[error(transparent)]
    CargoCmd(io::Error),
    #[error("exit status {0}. {hint}", hint = super::DOCTOR_HINT)]
    Exit(ExitStatus),
    #[error("package {package} not found")]
    PackageNotFound { package: String },
    #[error("creating out directory: {0}")]
    CreatingOutDir(io::Error),
    #[error("copying wasm file: {0}")]
    CopyingWasmFile(io::Error),
    #[error("getting the current directory: {0}")]
    GettingCurrentDir(io::Error),
    #[error(transparent)]
    StellarBuild(#[from] stellar_build::deps::Error),
    #[error(transparent)]
    BuildClients(#[from] clients::Error),
    #[error(transparent)]
    Build(#[from] build::Error),
    #[error("Failed to start docker container. {hint}", hint = super::DOCTOR_HINT)]
    DockerStart,
    #[error("package name is empty: {0}")]
    EmptyPackageName(Utf8PathBuf),
    #[error(transparent)]
    Config(#[from] v2::Error),
}

/// What `build` and `watch` read from the project's config before compiling.
pub struct Inputs {
    /// The resolved version 2 config, when building clients from one.
    pub plan: Option<env_toml::Environment>,
    /// Extensions to run, in order.
    pub extensions: Vec<env_toml::ExtensionEntry>,
    /// What hooks see as `env`: the environment or network name.
    pub env_label: String,
}

/// Start the local quickstart container if `network` runs locally.
async fn start_container_if_local(network: &env_toml::Network) -> Result<(), Error> {
    if !network.run_locally {
        return Ok(());
    }
    docker::start_local_stellar(network.rpc_url.as_deref())
        .await
        .map_err(|e| {
            eprintln!("Failed to start Stellar Docker container: {e:?}");
            Error::DockerStart
        })
}

impl Command {
    pub fn list_packages(&self, metadata: &Metadata) -> Result<Vec<Package>, Error> {
        let packages = self.packages(metadata)?;
        Ok(stellar_build::deps::get_workspace(&packages)?)
    }

    async fn start_local_docker_if_needed(
        &self,
        workspace_root: &Path,
        env: &ScaffoldEnv,
    ) -> Result<(), Error> {
        if let Some(current_env) = env_toml::Environment::get(workspace_root, env)? {
            start_container_if_local(&current_env.network).await?;
        }
        Ok(())
    }

    pub async fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);
        let metadata = self.metadata()?;
        let packages = self.list_packages(&metadata)?;
        let workspace_root = metadata.workspace_root.as_std_path();
        if !v2::is_v2(workspace_root) {
            self.start_development_container(workspace_root, &printer)
                .await?;
        }

        if self.list {
            for p in packages {
                println!("{}", p.name);
            }
            return Ok(());
        }

        let target_dir = &metadata.target_directory;

        // Configuration for this build: a resolved plan for version 2
        // projects, otherwise the selected environments.toml environment.
        let Inputs {
            plan,
            extensions,
            env_label,
        } = self
            .inputs(
                workspace_root,
                &metadata,
                &packages,
                &printer,
                self.build_clients,
            )
            .await?
            .unwrap_or_else(|| self.empty_inputs());
        let extensions = extension::discover(&extensions, &printer);
        // Build pre-compile context (wasm_paths is empty before compilation).
        let wasm_out_dir =
            self.build.out_dir.clone().unwrap_or_else(|| {
                stellar_build::deps::stellar_wasm_out_dir(target_dir.as_std_path())
            });
        let source_dirs: Vec<std::path::PathBuf> = packages
            .iter()
            .filter_map(|p| p.manifest_path.parent())
            .map(|p| p.as_std_path().to_path_buf())
            .collect();
        let pre_compile_ctx = CompileContext {
            config: None,
            project_root: workspace_root.to_path_buf(),
            env: env_label.clone(),
            wasm_out_dir: wasm_out_dir.clone(),
            source_dirs: source_dirs.clone(),
            wasm_paths: BTreeMap::new(),
        };

        extension::run_hook(
            &extensions,
            HookName::PreCompile,
            &pre_compile_ctx,
            &printer,
        )
        .await;

        for p in &packages {
            self.create_cmd(p, target_dir)?.run(global_args)?;
        }

        // Build post-compile context with populated wasm_paths.
        let wasm_paths: BTreeMap<String, std::path::PathBuf> = packages
            .iter()
            .map(|p| {
                let name = p.name.replace('-', "_");
                let path = stellar_build::stellar_wasm_out_file(target_dir.as_std_path(), &name);
                (name, path)
            })
            .collect();
        let post_compile_ctx = CompileContext {
            config: None,
            project_root: workspace_root.to_path_buf(),
            env: env_label,
            wasm_out_dir,
            source_dirs,
            wasm_paths,
        };

        extension::run_hook(
            &extensions,
            HookName::PostCompile,
            &post_compile_ctx,
            &printer,
        )
        .await;

        if self.build_clients {
            let mut build_clients_args = self.build_clients_args.clone();
            // Pass through the workspace_root, out_dir, global_args, and printer
            build_clients_args.workspace_root = Some(metadata.workspace_root.into_std_path_buf());
            build_clients_args.out_dir.clone_from(&self.build.out_dir);
            build_clients_args.global_args = Some(global_args.clone());
            build_clients_args.extensions = extensions;
            build_clients_args.compile_ctx = Some(post_compile_ctx);
            build_clients_args.plan = plan;
            build_clients_args
                .run(packages.iter().map(|p| p.name.replace('-', "_")).collect())
                .await?;
        }

        Ok(())
    }

    /// For environments.toml projects building `development`, start the local
    /// container if that environment runs locally. Version 2 projects start
    /// theirs while resolving the network.
    async fn start_development_container(
        &self,
        workspace_root: &Path,
        printer: &Print,
    ) -> Result<(), Error> {
        if self.build_clients_args.env == Some(ScaffoldEnv::Development) {
            printer.infoln("Starting local Stellar Docker container...");
            self.start_local_docker_if_needed(workspace_root, &ScaffoldEnv::Development)
                .await?;
            printer.checkln("Local Stellar network is healthy and running.");
        }
        Ok(())
    }

    /// Inputs for an environments.toml project with no entry for the selected
    /// environment: no extensions to run.
    fn empty_inputs(&self) -> Inputs {
        Inputs {
            plan: None,
            extensions: vec![],
            env_label: self.scaffold_env().to_string(),
        }
    }

    /// The environments.toml environment this build targets.
    fn scaffold_env(&self) -> ScaffoldEnv {
        self.build_clients_args
            .env
            .unwrap_or(ScaffoldEnv::Development)
    }

    /// The project's config inputs, from a version 2 `scaffold.yml` or from
    /// `environments.toml`. With `resolve_network`, a version 2 project also
    /// resolves the selected network for deploying (see `v2_inputs`).
    /// `None` when an environments.toml project has no entry for the
    /// selected environment.
    pub async fn inputs(
        &self,
        workspace_root: &Path,
        metadata: &Metadata,
        packages: &[Package],
        printer: &Print,
        resolve_network: bool,
    ) -> Result<Option<Inputs>, Error> {
        if v2::is_v2(workspace_root) {
            let inputs = self
                .v2_inputs(workspace_root, metadata, packages, printer, resolve_network)
                .await?;
            return Ok(Some(inputs));
        }
        let scaffold_env = self.scaffold_env();
        Ok(
            env_toml::Environment::get(workspace_root, &scaffold_env)?.map(|env| Inputs {
                plan: None,
                extensions: env.extensions,
                env_label: scaffold_env.to_string(),
            }),
        )
    }

    /// Inputs from a version 2 `scaffold.yml`. The file is always validated;
    /// with `resolve_network`, the selected network is also resolved, its
    /// container started if it runs locally, and any built `packages` that
    /// no contract lists are named as not being deployed.
    async fn v2_inputs(
        &self,
        workspace_root: &Path,
        metadata: &Metadata,
        packages: &[Package],
        printer: &Print,
        resolve_network: bool,
    ) -> Result<Inputs, Error> {
        if std::env::var_os("STELLAR_SCAFFOLD_ENV").is_some() {
            printer.warnln(
                "STELLAR_SCAFFOLD_ENV is ignored with a version 2 scaffold.yml; use --network or STELLAR_NETWORK",
            );
        }
        let crates = crate::config::cdylib_crates(metadata);
        let network_flag = self.build_clients_args.network.as_deref();
        if !resolve_network {
            let (extensions, env_label) =
                v2::hook_inputs(workspace_root, Some(&crates), network_flag, printer)?;
            return Ok(Inputs {
                plan: None,
                extensions,
                env_label,
            });
        }

        let (network, source) = v2::select_network(network_flag);
        printer.infoln(format!("Building for network {network:?} (from {source})"));
        let plan = v2::plan(workspace_root, Some(&crates), &network, printer)?;
        let unlisted = v2::unlisted_crates(packages.iter().map(|p| p.name.as_str()), &plan);
        if !unlisted.is_empty() {
            printer.infoln(format!(
                "Not deploying {}: not listed under `contracts:` in {}",
                unlisted.join(", "),
                crate::config::CONFIG_FILE
            ));
        }
        if plan.network.run_locally {
            printer.infoln("Starting local Stellar Docker container...");
            start_container_if_local(&plan.network).await?;
            printer.checkln("Local Stellar network is healthy and running.");
        }
        Ok(Inputs {
            extensions: plan.extensions.clone(),
            env_label: network,
            plan: Some(plan),
        })
    }

    fn packages(&self, metadata: &Metadata) -> Result<Vec<Package>, Error> {
        if let Some(package) = &self.build.package {
            let package = metadata
                .packages
                .iter()
                .find(|p| p.name == *package)
                .ok_or_else(|| Error::PackageNotFound {
                    package: package.clone(),
                })?
                .clone();
            let manifest_path = package.manifest_path.clone().into_std_path_buf();
            let mut contracts = stellar_build::deps::contract(&manifest_path)?;
            contracts.push(package);
            return Ok(contracts);
        }
        Ok(metadata
            .packages
            .iter()
            .filter(|p| {
                // Filter crates by those that build to cdylib (wasm)
                p.targets
                    .iter()
                    .any(|t| t.crate_types.iter().any(|c| c == "cdylib"))
            })
            .cloned()
            .collect())
    }

    pub(crate) fn metadata(&self) -> Result<Metadata, cargo_metadata::Error> {
        let mut cmd = MetadataCommand::new();
        cmd.no_deps();
        // Set the manifest path if one is provided, otherwise rely on the cargo
        // commands default behavior of finding the nearest Cargo.toml in the
        // current directory, or the parent directories above it.
        if let Some(manifest_path) = &self.build.manifest_path {
            cmd.manifest_path(manifest_path);
        }
        // Do not configure features on the metadata command, because we are
        // only collecting non-dependency metadata, features have no impact on
        // the output.
        cmd.exec()
    }

    fn create_cmd(&self, p: &Package, target_dir: &Utf8PathBuf) -> Result<Cmd, Error> {
        let mut cmd = self.build.clone();
        cmd.out_dir = cmd.out_dir.or_else(|| {
            Some(stellar_build::deps::stellar_wasm_out_dir(
                target_dir.as_std_path(),
            ))
        });

        // Name is required in Cargo toml, so it should fail regardless
        if p.name.is_empty() {
            return Err(EmptyPackageName(p.manifest_path.clone()));
        }

        cmd.package = Some(p.name.clone());

        let mut meta_map = BTreeMap::new();

        meta_map.insert("scaffold_version".to_string(), version::pkg().to_string());

        if let Value::Object(map) = &p.metadata
            && let Some(val) = &map.get("stellar")
            && let Value::Object(stellar_meta) = val
        {
            // When cargo_inherit is set, copy meta from Cargo toml
            if let Some(Value::Bool(true)) = stellar_meta.get("cargo_inherit") {
                meta_map.insert("name".to_string(), p.name.clone());

                if !p.version.to_string().is_empty() {
                    meta_map.insert("binver".to_string(), p.version.to_string());
                }
                if !p.authors.is_empty() {
                    meta_map.insert("authors".to_string(), p.authors.join(", "));
                }
                if let Some(homepage) = p.homepage.clone() {
                    meta_map.insert("homepage".to_string(), homepage);
                }
                if let Some(repository) = p.repository.clone() {
                    meta_map.insert("repository".to_string(), repository);
                }
            }
            Self::rec_add_meta(String::new(), &mut meta_map, val);
            // Reserved keys
            meta_map.remove("rsver");
            meta_map.remove("rssdkver");
            meta_map.remove("cargo_inherit");
            // Rename some fields
            if let Some(version) = meta_map.remove("version") {
                meta_map.insert("binver".to_string(), version);
            }
            if let Some(repository) = meta_map.remove("repository") {
                meta_map.insert("source_repo".to_string(), repository);
            }
            if let Some(homepage) = meta_map.remove("homepage") {
                meta_map.insert("home_domain".to_string(), homepage);
            }
        }
        cmd.build_args.meta.extend(meta_map);
        Ok(cmd)
    }

    fn rec_add_meta(prefix: String, meta_map: &mut BTreeMap<String, String>, value: &Value) {
        match value {
            Value::Null => {}
            Value::Bool(bool) => {
                meta_map.insert(prefix, bool.to_string());
            }
            Value::Number(n) => {
                meta_map.insert(prefix, n.to_string());
            }
            Value::String(s) => {
                meta_map.insert(prefix, s.clone());
            }
            Value::Array(array) => {
                if array.iter().all(Self::is_simple) {
                    let s = array
                        .iter()
                        .map(|x| match x {
                            Value::String(str) => str.clone(),
                            _ => x.to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    meta_map.insert(prefix, s);
                } else {
                    for (pos, e) in array.iter().enumerate() {
                        Self::rec_add_meta(format!("{prefix}[{pos}]"), meta_map, e);
                    }
                }
            }
            Value::Object(map) => {
                let mut separator = "";
                if !prefix.is_empty() {
                    separator = ".";
                }
                map.iter().for_each(|(k, v)| {
                    Self::rec_add_meta(format!("{prefix}{separator}{}", k.clone()), meta_map, v);
                });
            }
        }
    }

    fn is_simple(val: &Value) -> bool {
        !matches!(val, Value::Array(_) | Value::Object(_))
    }
}
