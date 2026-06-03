#![allow(clippy::struct_excessive_bools)]
use super::env_toml::Network;
use crate::arg_parsing;
use crate::arg_parsing::ArgParser;
use crate::commands::build::clients::Error::UpgradeArgsError;
use crate::commands::build::env_toml::{self, Environment};
use crate::commands::build::scaffold_yml::ScaffoldConfig;
use crate::commands::{PackageManager, PackageManagerSpec};
use crate::extension::{self, ResolvedExtension};
use indexmap::IndexMap;
use regex::Regex;
use serde_json;
use shlex::split;
use std::hash::Hash;
use std::path::Path;
use std::process::Command;
use std::{fmt::Debug, path::PathBuf};
use stellar_cli::{
    CommandParser, commands as cli,
    commands::contract::info::shared::{
        self as contract_spec, Args as FetchArgs, Error as FetchError, fetch,
    },
    config::{UnresolvedMuxedAccount, network, sign_with},
    print::Print,
    utils::contract_hash,
    utils::contract_spec::Spec,
};
use stellar_scaffold_ext_types::{
    CodegenContext, CompileContext, DeployContext, DeployKind, HookName, NetworkConfig,
};
use stellar_strkey::{self, Contract};
use stellar_xdr::curr::ScSpecEntry::FunctionV0;
use stellar_xdr::curr::{Error as xdrError, ScSpecEntry, ScSpecTypeBytesN, ScSpecTypeDef};

/// Internal decision about what deploy action to take for a contract.
/// Resolved before `pre-deploy` fires so the hook always has a clean execution context.
enum DeployDecision {
    /// No existing alias, or existing contract is not upgradeable — create a new instance.
    Fresh,
    /// Existing contract upgraded in-place; the returned ID is unchanged.
    Upgraded(Contract),
    /// Existing contract already has the target WASM hash — no on-chain action needed.
    Unchanged(Contract),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, clap::ValueEnum)]
pub enum ScaffoldEnv {
    Development,
    Testing,
    Staging,
    Production,
}
impl ScaffoldEnv {
    pub fn testing_or_development(&self) -> bool {
        matches!(self, ScaffoldEnv::Testing | ScaffoldEnv::Development)
    }
}

impl std::fmt::Display for ScaffoldEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}

#[derive(clap::Args, Clone, Debug)]
pub struct Args {
    #[arg(env = "STELLAR_SCAFFOLD_ENV", value_enum)]
    pub env: Option<ScaffoldEnv>,
    #[arg(skip)]
    pub workspace_root: Option<std::path::PathBuf>,
    /// Directory where wasm files are located
    #[arg(skip)]
    pub out_dir: Option<std::path::PathBuf>,
    #[arg(skip)]
    pub global_args: Option<stellar_cli::commands::global::Args>,
    #[arg(skip)]
    pub extensions: Vec<ResolvedExtension>,
    #[arg(skip)]
    pub compile_ctx: Option<CompileContext>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    EnvironmentsToml(#[from] env_toml::Error),
    #[error(
        "⛔ ️invalid network: must either specify a network name or both network_passphrase and rpc_url"
    )]
    MalformedNetwork,
    #[error(transparent)]
    ParsingNetwork(#[from] cli::network::Error),
    #[error(transparent)]
    GeneratingKey(#[from] cli::keys::generate::Error),
    #[error("⛔ ️can only have one default account; marked as default: {0:?}")]
    OnlyOneDefaultAccount(Vec<String>),
    #[error(transparent)]
    InvalidPublicKey(#[from] cli::keys::public_key::Error),
    #[error(transparent)]
    AddressParsing(#[from] stellar_cli::config::address::Error),
    #[error(
        "⛔ ️you need to provide at least one account, to use as the source account for contract deployment and other operations"
    )]
    NeedAtLeastOneAccount,
    #[error("⛔ ️No contract named {0:?}")]
    BadContractName(String),
    #[error("⛔ ️Invalid contract ID: {0:?}")]
    InvalidContractID(String),
    #[error("⛔ ️An ID must be set for a contract in production or staging. E.g. <name>.id = C...")]
    MissingContractID(String),
    #[error("⛔ ️Unable to parse script: {0:?}")]
    ScriptParseFailure(String),
    #[error(transparent)]
    RpcClient(#[from] soroban_rpc::Error),
    #[error("⛔ ️Failed to execute subcommand: {0:?}\n{1:?}")]
    SubCommandExecutionFailure(String, String),
    #[error(transparent)]
    ContractInstall(#[from] cli::contract::upload::Error),
    #[error(transparent)]
    ContractDeploy(#[from] cli::contract::deploy::wasm::Error),
    #[error(transparent)]
    ContractBindings(#[from] cli::contract::bindings::typescript::Error),
    #[error(transparent)]
    ContractFetch(#[from] cli::contract::fetch::Error),
    #[error(transparent)]
    ConfigLocator(#[from] stellar_cli::config::locator::Error),
    #[error(transparent)]
    ConfigNetwork(#[from] stellar_cli::config::network::Error),
    #[error(transparent)]
    ContractInvoke(#[from] cli::contract::invoke::Error),
    #[error(transparent)]
    ContractInfo(#[from] cli::contract::info::interface::Error),
    #[error(transparent)]
    Clap(#[from] clap::Error),
    #[error(transparent)]
    WasmHash(#[from] xdrError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("⛔ ️Failed to run package manager command in {0:?}: {1}")]
    PackageManagerCommandFailure(std::path::PathBuf, String),
    #[error("⛔ ️Codegen step for {0:?} failed: {1}")]
    CodegenStepFailed(String, String),
    #[error(transparent)]
    AccountFund(#[from] cli::keys::fund::Error),
    #[error("Failed to get upgrade operator: {0:?}")]
    UpgradeArgsError(arg_parsing::Error),
    #[error(transparent)]
    FetchError(#[from] FetchError),
    #[error(transparent)]
    SpecError(#[from] stellar_cli::get_spec::contract_spec::Error),
    #[error(transparent)]
    Strkey(#[from] stellar_strkey::DecodeError),
    #[error("Missing Workspace")]
    MissingWorkspace,
}

pub struct Builder {
    pub global_args: stellar_cli::commands::global::Args,
    pub network: network::Network,
    pub source_account: UnresolvedMuxedAccount,
    pub workspace_root: PathBuf,
    scaffold_env: ScaffoldEnv,
    printer: Print,
    pub(crate) out_dir: Option<PathBuf>,
    env: Environment,
    pkg_manager: PackageManager,
    extensions: Vec<ResolvedExtension>,
    compile_ctx: Option<CompileContext>,
    scaffold_config: ScaffoldConfig,
}

impl Builder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        global_args: stellar_cli::commands::global::Args,
        network: network::Network,
        source_account: UnresolvedMuxedAccount,
        workspace_root: PathBuf,
        scaffold_env: ScaffoldEnv,
        out_dir: Option<PathBuf>,
        env: Environment,
        pkg_manager: PackageManager,
        extensions: Vec<ResolvedExtension>,
        compile_ctx: Option<CompileContext>,
        scaffold_config: ScaffoldConfig,
    ) -> Self {
        Self {
            printer: Print::new(global_args.quiet),
            global_args,
            network,
            source_account,
            scaffold_env,
            workspace_root,
            out_dir,
            env,
            pkg_manager,
            extensions,
            compile_ctx,
            scaffold_config,
        }
    }

    fn config(&self) -> stellar_cli::config::Args {
        stellar_cli::config::Args {
            locator: self.global_args.locator.clone(),
            network: to_args(&self.network),
            sign_with: sign_with::Args::default(),
            source_account: self.source_account.clone(),
            fee: None,
            inclusion_fee: None,
        }
    }

    fn stellar_scaffold_env(&self) -> ScaffoldEnv {
        self.scaffold_env
    }

    fn get_config_locator(&self) -> &stellar_cli::config::locator::Args {
        &self.global_args.locator
    }

    fn get_config_dir(&self) -> Result<PathBuf, Error> {
        Ok(self.get_config_locator().config_dir()?)
    }

    fn printer(&self) -> &Print {
        &self.printer
    }

    fn network_config(&self) -> NetworkConfig {
        NetworkConfig {
            rpc_url: self.network.rpc_url.clone(),
            network_passphrase: self.network.network_passphrase.clone(),
            network_name: self.env.network.name.clone(),
        }
    }

    fn base_compile_ctx(&self) -> CompileContext {
        self.compile_ctx.clone().unwrap_or_else(|| {
            let target = self.workspace_root.join("target");
            CompileContext {
                config: None,
                project_root: self.workspace_root.clone(),
                env: self.scaffold_env.to_string(),
                wasm_out_dir: stellar_build::deps::stellar_wasm_out_dir(&target),
                source_dirs: vec![],
                wasm_paths: std::collections::BTreeMap::new(),
            }
        })
    }

    fn deploy_ctx(
        &self,
        name: &str,
        wasm_path: PathBuf,
        wasm_hash: &str,
        contract_id: Option<String>,
        deploy_kind: Option<DeployKind>,
    ) -> DeployContext {
        DeployContext {
            compile: self.base_compile_ctx(),
            network: self.network_config(),
            contract_name: name.to_string(),
            wasm_path,
            wasm_hash: wasm_hash.to_string(),
            contract_id,
            deploy_kind,
        }
    }

    fn codegen_ctx(
        &self,
        name: &str,
        contract_id: &str,
        wasm_hash: Option<&str>,
        ts_package_dir: PathBuf,
        src_template_path: PathBuf,
    ) -> CodegenContext {
        CodegenContext {
            deploy: self.deploy_ctx(
                name,
                self.get_wasm_path(name),
                wasm_hash.unwrap_or(""),
                Some(contract_id.to_string()),
                None,
            ),
            ts_package_dir,
            src_template_path,
        }
    }

    fn get_contract_alias(
        &self,
        name: &str,
        network: &network::Network,
    ) -> Result<Option<Contract>, stellar_cli::config::locator::Error> {
        self.get_config_locator()
            .get_contract_id(name, &network.network_passphrase)
    }

    async fn get_contract_hash(
        &self,
        contract_id: &Contract,
        network: &network::Network,
    ) -> Result<Option<String>, Error> {
        let fetch_cmd = cli::contract::fetch::Cmd {
            contract_id: Some(stellar_cli::config::UnresolvedContract::Resolved(
                *contract_id,
            )),
            out_file: None,
            locator: self.get_config_locator().clone(),
            network: to_args(network),
            wasm_hash: None,
        };
        let result = fetch_cmd.execute(&self.config()).await;

        match result {
            Ok(result) => {
                let ctrct_hash = contract_hash(&result)?;
                Ok(Some(hex::encode(ctrct_hash)))
            }
            Err(e) => {
                if e.to_string().contains("Contract not found") {
                    Ok(None)
                } else {
                    Err(Error::ContractFetch(e))
                }
            }
        }
    }

    fn save_contract_alias(
        &self,
        name: &str,
        contract_id: &Contract,
        network: &network::Network,
    ) -> Result<(), stellar_cli::config::locator::Error> {
        let config_dir = self.get_config_locator();
        let passphrase = &network.network_passphrase;
        config_dir.save_contract_id(passphrase, contract_id, name)
    }

    fn create_contract_template(
        &self,
        name: &str,
        contract_id: &str,
        network: &network::Network,
    ) -> Result<(), Error> {
        let allow_http = if self.stellar_scaffold_env().testing_or_development() {
            "\n  allowHttp: true,"
        } else {
            ""
        };
        let network_passphrase = &network.network_passphrase;
        let template = format!(
            r"import * as Client from '{name}';
import {{ rpcUrl }} from '@stellar-scaffold/ui-core';

export default new Client.Client({{
  networkPassphrase: '{network_passphrase}',
  contractId: '{contract_id}',
  rpcUrl,{allow_http}
  publicKey: undefined,
}});
"
        );
        let path = self
            .workspace_root
            .join(&self.scaffold_config.clients_dir)
            .join(format!("{name}.ts"));
        std::fs::write(path, template)?;
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    async fn generate_contract_bindings(
        &self,
        name: &str,
        contract_id: &str,
        wasm_hash: Option<&str>,
        rebuild: bool,
    ) -> Result<(), Error> {
        let network = &self.network;
        let printer = self.printer();
        let workspace_root = &self.workspace_root;
        let final_output_dir = workspace_root
            .join(&self.scaffold_config.bindings_dir)
            .join(name);
        let src_template_path = workspace_root
            .join(&self.scaffold_config.clients_dir)
            .join(format!("{name}.ts"));

        extension::run_hook(
            &self.extensions,
            HookName::PreCodegen,
            &self.codegen_ctx(
                name,
                contract_id,
                wasm_hash,
                final_output_dir.clone(),
                src_template_path.clone(),
            ),
            printer,
        )
        .await;

        let pm_label = self.pkg_manager.as_str();

        // Convert any inner error to a String *before* the PostCodegen await:
        // `Error` is not `Send` (some upstream variants wrap non-Send types),
        // and futures spawned via `tokio::spawn` (see commands/watch/mod.rs)
        // require everything held across an await to be Send.
        let codegen_failure: Option<String> = async {
            if rebuild {
                let temp_dir = workspace_root
                    .join("target")
                    .join(&self.scaffold_config.bindings_dir)
                    .join(name);
                let temp_dir_display = temp_dir.display();
                let config_dir = self.get_config_dir()?;

                let bindings_cmd = cli::contract::bindings::typescript::Cmd::parse_arg_vec(&[
                    "--contract-id",
                    contract_id,
                    "--output-dir",
                    temp_dir.to_str().expect("we do not support non-utf8 paths"),
                    "--config-dir",
                    config_dir
                        .to_str()
                        .expect("we do not support non-utf8 paths"),
                    "--overwrite",
                    "--rpc-url",
                    &network.rpc_url,
                    "--network-passphrase",
                    &network.network_passphrase,
                ])?;
                bindings_cmd.execute(true).await?;

                printer.infoln(format!(
                    "Running '{pm_label} install' in {temp_dir_display:?}"
                ));
                let output = self.pkg_manager.install_no_workspace(&temp_dir)?;

                if !output.status.success() {
                    let _ = std::fs::remove_dir_all(&temp_dir);
                    return Err(Error::PackageManagerCommandFailure(
                        temp_dir.clone(),
                        format!(
                            "{pm_label} install failed with status: {:?}\nError: {}",
                            output.status.code(),
                            String::from_utf8_lossy(&output.stderr)
                        ),
                    ));
                }
                printer.checkln(format!(
                    "'{pm_label} install' succeeded in {temp_dir_display}"
                ));

                printer.infoln(format!(
                    "Running '{pm_label} run build' in {temp_dir_display}"
                ));
                let output = self.pkg_manager.build(&temp_dir)?;

                if !output.status.success() {
                    let _ = std::fs::remove_dir_all(&temp_dir);
                    return Err(Error::PackageManagerCommandFailure(
                        temp_dir.clone(),
                        format!(
                            "{pm_label} run build failed with status: {:?}\nError: {}",
                            output.status.code(),
                            String::from_utf8_lossy(&output.stderr)
                        ),
                    ));
                }
                printer.checkln(format!(
                    "'{pm_label} run build' succeeded in {temp_dir_display}"
                ));

                if final_output_dir.exists() {
                    for p in ["dist/index.d.ts", "dist/index.js", "src/index.ts"]
                        .iter()
                        .map(Path::new)
                    {
                        std::fs::copy(temp_dir.join(p), final_output_dir.join(p))?;
                    }
                    printer.checkln(format!("Client {name:?} updated successfully"));
                } else {
                    std::fs::create_dir_all(&final_output_dir)?;
                    std::fs::rename(&temp_dir, &final_output_dir)?;
                    printer.checkln(format!("Client {name:?} created successfully"));
                    let output = self.pkg_manager.install_silent(&final_output_dir)?;

                    if !output.status.success() {
                        return Err(Error::PackageManagerCommandFailure(
                            final_output_dir.clone(),
                            format!(
                                "{pm_label} install in final directory failed with status: {:?}\nError: {}",
                                output.status.code(),
                                String::from_utf8_lossy(&output.stderr)
                            ),
                        ));
                    }
                }

                self.create_contract_template(name, contract_id, network)?;
            }
            Ok::<(), Error>(())
        }
        .await
        .err()
        .map(|e| e.to_string());

        // Fire PostCodegen unconditionally so hook consumers see a consistent
        // lifecycle even when an inner step errored.
        extension::run_hook(
            &self.extensions,
            HookName::PostCodegen,
            &self.codegen_ctx(
                name,
                contract_id,
                wasm_hash,
                final_output_dir,
                src_template_path,
            ),
            printer,
        )
        .await;

        if let Some(msg) = codegen_failure {
            return Err(Error::CodegenStepFailed(name.to_string(), msg));
        }
        Ok(())
    }

    async fn handle_accounts(&self) -> Result<(), Error> {
        let printer = self.printer();
        let network = &self.network;
        let accounts = self.env.accounts.as_deref();
        let Some(accounts) = accounts else {
            return Err(Error::NeedAtLeastOneAccount);
        };

        let config = self.get_config_locator();
        let args = &self.global_args;
        for account in accounts {
            printer.infoln(format!("Creating keys for {:?}", account.name));
            // Use provided global args or create default

            let generate_cmd = cli::keys::generate::Cmd {
                name: account.name.clone().parse()?,
                fund: true,
                config_locator: config.clone(),
                network: to_args(network),
                seed: None,
                hd_path: None,
                as_secret: false,
                secure_store: false,
                overwrite: false,
            };

            match generate_cmd.run(args).await {
                Err(e) if e.to_string().contains("already exists") => {
                    printer.blankln(e);
                    // Check if account exists on chain
                    let rpc_client = soroban_rpc::Client::new(&network.rpc_url)?;

                    let public_key_cmd = cli::keys::public_key::Cmd {
                        name: account.name.parse()?,
                        locator: config.clone(),
                        hd_path: None,
                    };
                    let address = public_key_cmd.public_key().await?;

                    if (rpc_client.get_account(&address.to_string()).await).is_err() {
                        printer.infoln("Account not found on chain, funding...");
                        let fund_cmd = cli::keys::fund::Cmd {
                            network: to_args(network),
                            address: public_key_cmd,
                        };
                        fund_cmd.run(args).await?;
                    }
                }
                other_result => other_result?,
            }
        }
        Ok(())
    }

    fn maintain_user_ordering(
        package_names: &[String],
        contracts: Option<&IndexMap<Box<str>, env_toml::Contract>>,
    ) -> Vec<String> {
        contracts.map_or_else(
            || package_names.to_vec(),
            |contracts| {
                let mut reordered: Vec<String> = contracts
                    .keys()
                    .filter_map(|contract_name| {
                        package_names
                            .iter()
                            .find(|&name| name == contract_name.as_ref())
                            .cloned()
                    })
                    .collect();

                reordered.extend(
                    package_names
                        .iter()
                        .filter(|name| !contracts.contains_key(name.as_str()))
                        .cloned(),
                );

                reordered
            },
        )
    }

    async fn handle_production_contracts(
        &self,
        contracts: &IndexMap<Box<str>, env_toml::Contract>,
    ) -> Result<(), Error> {
        for (name, contract) in contracts.iter().filter(|(_, settings)| settings.client) {
            if let Some(id) = &contract.id {
                if stellar_strkey::Contract::from_string(id).is_err() {
                    return Err(Error::InvalidContractID(id.clone()));
                }
                self.generate_contract_bindings(name, id, None, true)
                    .await?;
            } else {
                return Err(Error::MissingContractID(name.to_string()));
            }
        }
        Ok(())
    }

    async fn handle_contracts(&self, package_names: Vec<String>) -> Result<(), Error> {
        let printer = self.printer();
        if package_names.is_empty() {
            return Ok(());
        }
        let contracts = self.env.contracts.as_ref();
        let network = &self.network;
        let env = self.stellar_scaffold_env();
        if matches!(env, ScaffoldEnv::Production | ScaffoldEnv::Staging) {
            if let Some(contracts) = contracts {
                self.handle_production_contracts(contracts).await?;
            }
            return Ok(());
        }

        self.validate_contract_names(contracts)?;

        let names = Self::maintain_user_ordering(&package_names, contracts);

        for name in names {
            let settings = contracts
                .and_then(|contracts| contracts.get(name.as_str()))
                .cloned()
                .unwrap_or_default();

            // Skip if client generation is disabled
            if !settings.client {
                continue;
            }

            match self
                .process_single_contract(&name, settings, network, env)
                .await
            {
                Ok(()) => {
                    printer.checkln(format!("Successfully generated client for: {name}"));
                }
                Err(e) => {
                    printer.errorln(format!("Failed to generate client for: {name}: {e}"));
                }
            }
        }

        Ok(())
    }

    fn get_wasm_path(&self, contract_name: &str) -> std::path::PathBuf {
        // Check if out_dir was specified and use it, otherwise fall back to target directory
        if let Some(out_dir) = &self.out_dir {
            out_dir.join(format!("{contract_name}.wasm"))
        } else {
            let workspace_root = &self.workspace_root;
            let target_dir = workspace_root.join("target");
            stellar_build::stellar_wasm_out_file(&target_dir, contract_name)
        }
    }

    fn validate_contract_names(
        &self,
        contracts: Option<&IndexMap<Box<str>, env_toml::Contract>>,
    ) -> Result<(), Error> {
        let Some(contracts) = contracts else {
            return Ok(());
        };
        for (name, _) in contracts.iter().filter(|(_, settings)| settings.client) {
            let wasm_path = self.get_wasm_path(name);
            if !wasm_path.exists() {
                return Err(Error::BadContractName(name.to_string()));
            }
        }
        Ok(())
    }

    async fn process_single_contract(
        &self,
        name: &str,
        settings: env_toml::Contract,
        network: &network::Network,
        env: ScaffoldEnv,
    ) -> Result<(), Error> {
        let printer = self.printer();
        // Returns (contract_id, wasm_hash, needs_rebuild).
        // wasm_hash is None for pinned-ID contracts; needs_rebuild is always true for them.
        let (contract_id, wasm_hash, needs_rebuild) = if let Some(id) = &settings.id {
            let contract_id =
                Contract::from_string(id).map_err(|_| Error::InvalidContractID(id.clone()))?;
            (contract_id, None::<String>, true)
        } else {
            let wasm_path = self.get_wasm_path(name);
            if !wasm_path.exists() {
                return Err(Error::BadContractName(name.to_string()));
            }
            let new_hash = self.upload_contract_wasm(name, &wasm_path).await?;

            // Determine what deploy action is needed before firing any hooks.
            let decision = if let Some(existing_id) = self.get_contract_alias(name, network)? {
                let hash = self.get_contract_hash(&existing_id, network).await?;
                if let Some(current_hash) = hash {
                    if current_hash == new_hash {
                        printer.checkln(format!("Contract {name:?} is up to date"));
                        DeployDecision::Unchanged(existing_id)
                    } else {
                        match self
                            .try_upgrade_contract(
                                name,
                                existing_id,
                                &current_hash,
                                &new_hash,
                                network,
                            )
                            .await?
                        {
                            Some(upgraded_id) => DeployDecision::Upgraded(upgraded_id),
                            None => DeployDecision::Fresh,
                        }
                    }
                } else {
                    DeployDecision::Fresh
                }
            } else {
                DeployDecision::Fresh
            };

            extension::run_hook(
                &self.extensions,
                HookName::PreDeploy,
                &self.deploy_ctx(name, wasm_path.clone(), &new_hash, None, None),
                printer,
            )
            .await;

            let (contract_id, deploy_kind) = match decision {
                DeployDecision::Unchanged(id) => (id, DeployKind::Unchanged),
                DeployDecision::Upgraded(id) => (id, DeployKind::Upgraded),
                DeployDecision::Fresh => {
                    let id = self.deploy_contract(name, &new_hash, &settings).await?;
                    (id, DeployKind::Fresh)
                }
            };

            // Run after_deploy script and save alias only when something changed on-chain.
            if deploy_kind != DeployKind::Unchanged {
                if let Some(after_deploy) = settings.after_deploy.as_deref()
                    && (env == ScaffoldEnv::Development || env == ScaffoldEnv::Testing)
                {
                    printer.infoln(format!("Running after_deploy script for {name:?}"));
                    self.run_after_deploy_script(name, &contract_id, after_deploy)
                        .await?;
                }
                self.save_contract_alias(name, &contract_id, network)?;
            }

            extension::run_hook(
                &self.extensions,
                HookName::PostDeploy,
                &self.deploy_ctx(
                    name,
                    wasm_path,
                    &new_hash,
                    Some(contract_id.to_string()),
                    Some(deploy_kind.clone()),
                ),
                printer,
            )
            .await;

            let needs_rebuild = deploy_kind != DeployKind::Unchanged
                || !self
                    .workspace_root
                    .join(&self.scaffold_config.bindings_dir)
                    .join(name)
                    .exists();
            (contract_id, Some(new_hash), needs_rebuild)
        };

        self.generate_contract_bindings(
            name,
            &contract_id.to_string(),
            wasm_hash.as_deref(),
            needs_rebuild,
        )
        .await?;

        Ok(())
    }

    async fn upload_contract_wasm(
        &self,
        _name: &str,
        wasm_path: &std::path::Path,
    ) -> Result<String, Error> {
        let cmd = cli::contract::upload::Cmd {
            config: self.config(),
            resources: stellar_cli::resources::Args::default(),
            wasm: Some(wasm_path.to_path_buf()),
            ignore_checks: false,
            build_only: false,
            package: None,
            build_args: cli::contract::build::BuildArgs::default(),
        };
        let hash = cmd
            .execute(
                &cmd.config,
                self.global_args.quiet,
                self.global_args.no_cache,
            )
            .await?
            .into_result()
            .expect("no hash returned by 'contract upload'")
            .to_string();
        Ok(hash)
    }

    fn parse_script_line(line: &str) -> Result<(Option<String>, Vec<String>), Error> {
        let re = Regex::new(r"\$\((.*?)\)").expect("Invalid regex pattern");
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let resolved_line = Self::resolve_line(&re, line, shell, flag)?;
        let parts = split(&resolved_line)
            .ok_or_else(|| Error::ScriptParseFailure(resolved_line.clone()))?;

        let (source_account, command_parts): (Vec<_>, Vec<_>) = parts
            .into_iter()
            .partition(|part| part.starts_with("STELLAR_ACCOUNT="));

        let source = source_account.first().map(|account: &String| {
            account
                .strip_prefix("STELLAR_ACCOUNT=")
                .unwrap()
                .to_string()
        });

        Ok((source, command_parts))
    }

    async fn deploy_contract(
        &self,
        name: &str,
        hash: &str,
        settings: &env_toml::Contract,
    ) -> Result<Contract, Error> {
        let source = self.source_account.to_string();
        let mut deploy_args = vec![
            format!("--alias={name}"),
            format!("--wasm-hash={hash}"),
            "--config-dir".to_string(),
            self.get_config_dir()?
                .to_str()
                .expect("we do not support non-utf8 paths")
                .to_string(),
        ];
        if let Some(constructor_script) = &settings.constructor_args {
            let (source_account, mut args) = Self::parse_script_line(constructor_script)?;

            if let Some(account) = source_account {
                deploy_args.extend_from_slice(&["--source-account".to_string(), account]);
            } else {
                deploy_args.extend_from_slice(&["--source".to_string(), source]);
            }

            deploy_args.push("--".to_string());
            deploy_args.append(&mut args);
        } else {
            deploy_args.extend_from_slice(&["--source".to_string(), source]);
        }

        let deploy_arg_refs: Vec<&str> = deploy_args
            .iter()
            .map(std::string::String::as_str)
            .collect();
        let deploy_cmd = cli::contract::deploy::wasm::Cmd::parse_arg_vec(&deploy_arg_refs)?;
        let contract_id = deploy_cmd
            .execute(
                &self.config(),
                self.global_args.quiet,
                self.global_args.no_cache,
            )
            .await?
            .into_result()
            .expect("no contract id returned by 'contract deploy'");

        Ok(contract_id)
    }

    async fn try_upgrade_contract(
        &self,
        name: &str,
        existing_contract_id: Contract,
        existing_hash: &str,
        hash: &str,
        network: &network::Network,
    ) -> Result<Option<Contract>, Error> {
        let printer = self.printer();
        let existing_spec = fetch_contract_spec(existing_hash, network).await?;
        let spec_to_upgrade = fetch_contract_spec(hash, network).await?;
        let Some(legacy_upgradeable) = Self::is_legacy_upgradeable(&existing_spec) else {
            return Ok(None);
        };

        if Self::is_legacy_upgradeable(&spec_to_upgrade).is_none() {
            printer.warnln("New WASM is not upgradable. Contract will be redeployed instead of being upgraded.");
            return Ok(None);
        }

        let existing_contract_id_str = existing_contract_id.to_string();
        let source = self.source_account.to_string();
        let mut redeploy_args = vec![
            "--source",
            source.as_str(),
            "--id",
            existing_contract_id_str.as_str(),
            "--",
            "upgrade",
            "--new_wasm_hash",
            hash,
        ];
        let invoke_cmd = if legacy_upgradeable {
            let upgrade_operator = ArgParser::get_upgrade_args(name).map_err(UpgradeArgsError)?;
            redeploy_args.push("--operator");
            redeploy_args.push(&upgrade_operator);
            cli::contract::invoke::Cmd::parse_arg_vec(&redeploy_args)
        } else {
            cli::contract::invoke::Cmd::parse_arg_vec(&redeploy_args)
        }?;
        invoke_cmd
            .execute(
                &self.config(),
                self.global_args.quiet,
                self.global_args.no_cache,
            )
            .await?
            .into_result()
            .expect("no result returned by 'contract invoke'");

        Ok(Some(existing_contract_id))
    }

    /// Returns: none if not upgradable, Some(true) if legacy upgradeable, Some(false) if new upgradeable
    fn is_legacy_upgradeable(spec: &[ScSpecEntry]) -> Option<bool> {
        spec.iter()
            .filter_map(|x| if let FunctionV0(e) = x { Some(e) } else { None })
            .filter(|x| x.name.to_string() == "upgrade")
            .find(|x| {
                x.inputs
                    .iter()
                    .any(|y| matches!(y.type_, ScSpecTypeDef::BytesN(ScSpecTypeBytesN { n: 32 })))
            })
            .map(|x| x.inputs.iter().any(|y| y.type_ == ScSpecTypeDef::Address))
    }

    fn resolve_line(re: &Regex, line: &str, shell: &str, flag: &str) -> Result<String, Error> {
        let mut result = String::new();
        let mut last_match = 0;
        for cap in re.captures_iter(line) {
            let whole_match = cap.get(0).unwrap();
            result.push_str(&line[last_match..whole_match.start()]);
            let cmd = &cap[1];
            let output = Self::execute_subcommand(shell, flag, cmd)?;
            result.push_str(&output);
            last_match = whole_match.end();
        }
        result.push_str(&line[last_match..]);
        Ok(result)
    }

    fn execute_subcommand(shell: &str, flag: &str, cmd: &str) -> Result<String, Error> {
        match Command::new(shell).arg(flag).arg(cmd).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

                if output.status.success() {
                    Ok(stdout)
                } else {
                    Err(Error::SubCommandExecutionFailure(cmd.to_string(), stderr))
                }
            }
            Err(e) => Err(Error::SubCommandExecutionFailure(
                cmd.to_string(),
                e.to_string(),
            )),
        }
    }

    async fn run_after_deploy_script(
        &self,
        name: &str,
        contract_id: &Contract,
        after_deploy_script: &str,
    ) -> Result<(), Error> {
        let printer = self.printer();
        let config_dir_path = self.get_config_dir()?;
        let config_dir = config_dir_path.to_str().unwrap();
        let source = self.source_account.to_string();
        for line in after_deploy_script.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let (source_account, command_parts) = Self::parse_script_line(line)?;

            let contract_id_arg = contract_id.to_string();
            let mut args = vec!["--id", &contract_id_arg, "--config-dir", config_dir];
            if let Some(account) = source_account.as_ref() {
                args.extend_from_slice(&["--source-account", account]);
            } else {
                args.extend_from_slice(&["--source-account", source.as_str()]);
            }
            args.extend_from_slice(&["--"]);
            args.extend(command_parts.iter().map(std::string::String::as_str));

            printer.infoln(format!(
                "  ↳ Executing: stellar contract invoke {}",
                args.join(" ")
            ));
            let invoke_cmd = cli::contract::invoke::Cmd::parse_arg_vec(&args)?;
            let result = invoke_cmd
                .execute(
                    &self.config(),
                    self.global_args.quiet,
                    self.global_args.no_cache,
                )
                .await?;
            printer.infoln(format!("  ↳ Result: {result:?}"));
        }
        printer.checkln(format!(
            "After deploy script for {name:?} completed successfully"
        ));
        Ok(())
    }
}

impl Args {
    fn printer(&self) -> Print {
        Print::new(self.global_args.as_ref().is_some_and(|args| args.quiet))
    }

    pub fn builder(&self) -> Result<Builder, Error> {
        let workspace_root = self
            .workspace_root
            .as_ref()
            .expect("workspace_root must be set before running");
        let env = self.env.unwrap_or(ScaffoldEnv::Development);
        let global_args = self.global_args.clone().unwrap_or_default();

        let Some(current_env) = env_toml::Environment::get(workspace_root, &env)? else {
            return Err(Error::MissingWorkspace);
        };
        let network = to_network(&global_args, current_env.network.clone())?;
        self.printer()
            .infoln(format!("Using network at {}\n", network.rpc_url));
        let accounts = current_env.accounts.clone().unwrap_or_default();
        let default_account_candidates = accounts
            .iter()
            .filter(|&account| account.default)
            .map(|account| account.name.clone())
            .collect::<Vec<_>>();

        let default_account = match (default_account_candidates.as_slice(), accounts.as_slice()) {
            ([], []) => return Err(Error::NeedAtLeastOneAccount),
            ([], [env_toml::Account { name, .. }, ..]) => name.clone(),
            ([candidate], _) => candidate.clone(),
            _ => return Err(Error::OnlyOneDefaultAccount(default_account_candidates)),
        };

        let pkg_manager_spec =
            PackageManagerSpec::from_package_json(workspace_root).unwrap_or(PackageManagerSpec {
                kind: PackageManager::Npm,
                version: None,
            });

        let scaffold_config = ScaffoldConfig::get(workspace_root);

        let builder = Builder::new(
            global_args,
            network,
            default_account.parse()?,
            workspace_root.clone(),
            env,
            self.out_dir.clone(),
            current_env,
            pkg_manager_spec.kind,
            self.extensions.clone(),
            self.compile_ctx.clone(),
            scaffold_config,
        );
        Ok(builder)
    }

    pub async fn run(&self, package_names: Vec<String>) -> Result<(), Error> {
        let builder = match self.builder() {
            Ok(builder) => builder,
            Err(Error::MissingWorkspace) => {
                return Ok(());
            }
            Err(e) => {
                return Err(e);
            }
        };
        builder.handle_accounts().await?;
        builder.handle_contracts(package_names).await?;
        Ok(())
    }
}

fn to_network(
    global: &stellar_cli::commands::global::Args,
    Network {
        name,
        rpc_url,
        network_passphrase,
        rpc_headers,
        ..
    }: env_toml::Network,
) -> Result<network::Network, network::Error> {
    network::Args {
        network: name,
        rpc_url,
        network_passphrase,
        rpc_headers: rpc_headers.unwrap_or_default(),
    }
    .get(&global.locator)
}

fn to_args(
    network::Network {
        rpc_url,
        rpc_headers,
        network_passphrase,
    }: &network::Network,
) -> network::Args {
    network::Args {
        network: None,
        network_passphrase: Some(network_passphrase.clone()),
        rpc_headers: rpc_headers.clone(),
        rpc_url: Some(rpc_url.clone()),
    }
}

async fn fetch_contract_spec(
    wasm_hash: &str,
    network: &network::Network,
) -> Result<Vec<ScSpecEntry>, Error> {
    let fetched = fetch(
        &FetchArgs {
            wasm_hash: Some(wasm_hash.to_string()),
            network: to_args(network),
            ..Default::default()
        },
        // Quiets the output of the fetch command
        &Print::new(true),
    )
    .await?;

    match fetched.contract {
        contract_spec::Contract::Wasm { wasm_bytes } => Ok(Spec::new(&wasm_bytes)?.spec),
        contract_spec::Contract::StellarAssetContract => unreachable!(),
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tempfile::TempDir;

//     #[test]
//     fn test_get_package_dir() {
//         let temp_dir = TempDir::new().unwrap();
//         let package_path = temp_dir.path().join("packages/existing_package");
//         std::fs::create_dir_all(&package_path).unwrap();
//         let args = Args {
//             env: Some(ScaffoldEnv::Development),
//             workspace_root: Some(temp_dir.path().to_path_buf()),
//             out_dir: None,
//             global_args: None,
//         };
//         let args = args.builder().unwrap();
//         let result = args.get_package_dir("existing_package");
//         assert!(result.is_ok());
//         let path = result.unwrap();
//         assert_eq!(path.file_name().unwrap(), "existing_package");
//     }

//     #[test]
//     fn test_get_package_dir_nonexistent() {
//         let args = Args {
//             env: Some(ScaffoldEnv::Development),
//             workspace_root: Some(std::path::PathBuf::from("tests/nonexistent_workspace")),
//             out_dir: None,
//             global_args: None,
//         };
//         let args = args.builder().unwrap();
//         let result = args.get_package_dir("nonexistent_package");
//         assert!(result.is_err());
//         if let Err(Error::BadContractName(name)) = result {
//             assert_eq!(name, "nonexistent_package");
//         } else {
//             panic!("Expected BadContractName error");
//         }
//     }
// }
