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
pub mod scaffold_yml;

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
    #[error("exit status {0}")]
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
    #[error("Failed to start docker container")]
    DockerStart,
    #[error("package name is empty: {0}")]
    EmptyPackageName(Utf8PathBuf),
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
        if let Some(current_env) = env_toml::Environment::get(workspace_root, env)?
            && current_env.network.run_locally
        {
            docker::start_local_stellar().await.map_err(|e| {
                eprintln!("Failed to start Stellar Docker container: {e:?}");
                Error::DockerStart
            })?;
        }
        Ok(())
    }

    pub async fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);
        let metadata = self.metadata()?;
        let packages = self.list_packages(&metadata)?;
        let workspace_root = metadata.workspace_root.as_std_path();

        if let Some(env) = &self.build_clients_args.env
            && env == &ScaffoldEnv::Development
        {
            printer.infoln("Starting local Stellar Docker container...");
            self.start_local_docker_if_needed(workspace_root, env)
                .await?;
            printer.checkln("Local Stellar network is healthy and running.");
        }

        if self.list {
            for p in packages {
                println!("{}", p.name);
            }
            return Ok(());
        }

        let target_dir = &metadata.target_directory;

        // Discover extensions for the active environment
        let scaffold_env = self
            .build_clients_args
            .env
            .unwrap_or(ScaffoldEnv::Development);
        let extensions = match env_toml::Environment::get(workspace_root, &scaffold_env)? {
            Some(env_config) if !env_config.extensions.is_empty() => {
                extension::discover(&env_config.extensions, &printer)
            }
            _ => vec![],
        };
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
            env: scaffold_env.to_string(),
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
            env: scaffold_env.to_string(),
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
            build_clients_args
                .run(packages.iter().map(|p| p.name.replace('-', "_")).collect())
                .await?;
        }

        Ok(())
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
