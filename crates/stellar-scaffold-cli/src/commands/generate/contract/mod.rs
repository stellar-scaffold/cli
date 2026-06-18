use cargo_toml::Dependency::Simple;
use cargo_toml::Inheritable::{Inherited, Set};
use cargo_toml::{
    Dependency, DepsSet, InheritedDependencyDetail, Manifest, Product, Publish, Workspace,
};
use clap::Parser;
use flate2::read::GzDecoder;
use reqwest;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::process::Command;
use std::{fs, path::Path};
use stellar_cli::commands::global;
use stellar_cli::print::Print;
use tar::Archive;
use toml::Value::Table;

const SOROBAN_EXAMPLES_REPO: &str = "https://github.com/stellar/soroban-examples";
const STELLAR_PREFIX: &str = "stellar/";
const OZ_EXAMPLES_REPO: &str = "https://github.com/OpenZeppelin/stellar-contracts/examples";
const OZ_PREFIX: &str = "oz/";
const LATEST_SUPPORTED_OZ_RELEASE: &str = "v0.7.2";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

#[derive(Parser, Debug)]
pub struct Cmd {
    /// Clone contract from `OpenZeppelin` examples or `soroban-examples`
    #[arg(long, conflicts_with_all = ["ls", "from_wizard"])]
    pub from: Option<String>,

    /// List available contract examples
    #[arg(long, conflicts_with_all = ["from", "from_wizard"])]
    pub ls: bool,

    /// Open contract generation wizard in browser
    #[arg(long, conflicts_with_all = ["from", "ls"])]
    pub from_wizard: bool,

    /// Output directory for the generated contract (defaults to contracts/<example-name>)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Force add contract to existing project (ignoring some errors)
    #[arg(long, conflicts_with_all = ["ls", "from_wizard"])]
    pub force: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    CargoToml(#[from] cargo_toml::Error),
    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Git command failed: {0}")]
    GitCloneFailed(String),
    #[error("Example '{0}' not found")]
    ExampleNotFound(String),
    #[error("Example '{0}' not found in OpenZeppelin stellar-contracts")]
    OzExampleNotFound(String),
    #[error("Example '{0}' not found in Stellar soroban-examples")]
    StellarExampleNotFound(String),
    #[error(
        "Invalid Cargo toml file for soroban-example {0}: missing [package] or [dependencies] sections"
    )]
    InvalidCargoToml(String),
    #[error(
        "Invalid workspace toml file in the root of the current directory: missing {0} section\nPlease make sure to run this command from the root of a Scaffold project."
    )]
    InvalidWorkspaceCargoToml(String),
    #[error("Failed to open browser: {0}")]
    BrowserFailed(String),
    #[error("No action specified. Use --from, --ls, or --from-wizard")]
    NoActionSpecified,
    #[error("Destination path {0} already exists. Use --force to overwrite it")]
    PathExists(PathBuf),
    #[error("Failed to update examples cache")]
    UpdateExamplesCache,
    #[error("Failed to fetch workspace Cargo.toml")]
    CargoError,
    #[error(
        "Dependency version mismatch for {0}: example version {1} doesn't match manifest version {2}"
    )]
    DependencyVersionMismatch(String, u32, u32),
    #[error("Missing workspace package")]
    MissingWorkspacePackage,
}

impl Cmd {
    pub async fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        match (&self.from, self.ls, self.from_wizard) {
            (Some(example_name), _, _) => self.clone_example(example_name, global_args).await,
            (_, true, _) => self.list_examples(global_args).await,
            (_, _, true) => open_wizard(global_args),
            _ => Err(Error::NoActionSpecified),
        }
    }

    async fn clone_example(
        &self,
        example_name: &str,
        global_args: &global::Args,
    ) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        printer.infoln(format!("Downloading example '{example_name}'..."));

        let examples_info = self.ensure_cache_updated(&printer).await?;

        if example_name.starts_with(OZ_PREFIX) {
            let (_, example_name) = example_name.split_at(3);
            let dest_path = self.output_dir(example_name);
            Self::generate_oz_example(
                example_name,
                &examples_info.oz_examples_path,
                &examples_info.oz_version_tag,
                &dest_path,
                global_args,
                &printer,
            )
        } else if example_name.starts_with(STELLAR_PREFIX) {
            let (_, example_name) = example_name.split_at(8);
            let dest_path = self.output_dir(example_name);
            self.generate_soroban_example(
                example_name,
                &examples_info.soroban_examples_path,
                &dest_path,
                &printer,
            )
        } else {
            Err(Error::ExampleNotFound(example_name.to_owned()))
        }
    }

    fn generate_oz_example(
        example_name: &str,
        repo_cache_path: &Path,
        tag_name: &str,
        dest_path: &Path,
        global_args: &global::Args,
        printer: &Print,
    ) -> Result<(), Error> {
        // Check if the example exists
        let example_source_path = repo_cache_path.join(format!("examples/{example_name}"));
        if !example_source_path.exists() {
            return Err(Error::OzExampleNotFound(example_name.to_string()));
        }

        // Create destination and copy example contents
        fs::create_dir_all(dest_path)?;
        Self::copy_directory_contents(&example_source_path, Path::new(&dest_path))?;

        // Read and update workspace Cargo.toml
        // Use dest_path (the project's contract directory) rather than
        // example_source_path (the cache) so that `cargo locate-project
        // --workspace` finds the project's Cargo.toml, not the cached
        // OZ repo's workspace.
        let workspace_cargo_path = Self::get_workspace_root(&dest_path.join("Cargo.toml"));
        if let Ok(workspace_cargo_path) = workspace_cargo_path {
            Self::update_workspace_dependencies(
                &workspace_cargo_path,
                &example_source_path,
                tag_name,
                global_args,
            )?;
        } else {
            printer.warnln("Warning: No workspace Cargo.toml found in current directory.");
            printer
                .println("   You'll need to manually add required dependencies to your workspace.");
        }

        printer.checkln(format!(
            "Successfully downloaded example '{example_name}' to {}",
            dest_path.display()
        ));
        printer
            .infoln("You may need to modify your environments.toml to add constructor arguments!");
        Ok(())
    }

    fn generate_soroban_example(
        &self,
        example_name: &str,
        repo_cache_path: &Path,
        dest_path: &Path,
        printer: &Print,
    ) -> Result<(), Error> {
        // Check if the example exists
        let example_source_path = repo_cache_path.join(example_name);
        if !example_source_path.exists() {
            return Err(Error::StellarExampleNotFound(example_name.to_string()));
        }
        if dest_path.exists() {
            if self.force {
                printer.warnln(format!(
                    "Overwriting existing directory {}...",
                    dest_path.display()
                ));
                fs::remove_dir_all(dest_path)?;
            } else {
                return Err(Error::PathExists(dest_path.to_owned()));
            }
        }

        // Create destination and copy example contents
        fs::create_dir_all(dest_path)?;
        Self::copy_directory_contents(&example_source_path, Path::new(&dest_path))?;

        match fs::remove_file(dest_path.join("Cargo.lock")) {
            Ok(..) => {}
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    printer.errorln(format!("Failed to remove Cargo.lock: {e}"));
                }
            }
        }
        match fs::remove_file(dest_path.join("Makefile")) {
            Ok(..) => {}
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    printer.errorln(format!("Failed to remove Makefile: {e}"));
                }
            }
        }

        let example_toml_path = dest_path.join("Cargo.toml");

        let workspace_cargo_path = Self::get_workspace_root(&example_toml_path);
        let Ok(workspace_cargo_path) = workspace_cargo_path else {
            printer.warnln("Warning: No workspace Cargo.toml found in current directory.");
            printer.println("You'll need to manually add contracts to your workspace.");
            return Ok(());
        };

        self.write_new_manifest(
            &workspace_cargo_path,
            &example_toml_path,
            example_name,
            printer,
        )?;

        printer.checkln(format!(
            "Successfully downloaded example '{example_name}' to {}",
            dest_path.display()
        ));
        printer
            .infoln("You may need to modify your environments.toml to add constructor arguments!");
        Ok(())
    }

    fn write_new_manifest(
        &self,
        workspace_toml_path: &Path,
        example_toml_path: &Path,
        example_name: &str,
        printer: &Print,
    ) -> Result<(), Error> {
        let workspace_manifest = Manifest::from_path(workspace_toml_path)?;
        let Some(workspace) = workspace_manifest.workspace.as_ref() else {
            return Err(Error::InvalidWorkspaceCargoToml(
                "[workspace.package]".to_string(),
            ));
        };
        let Some(workspace_package) = &workspace.package else {
            return Err(Error::MissingWorkspacePackage);
        };

        // Parse the Cargo.toml
        let manifest = cargo_toml::Manifest::from_path(example_toml_path)?;

        let package = manifest
            .package
            .ok_or(Error::InvalidCargoToml(example_name.to_string()))?;
        let name = package.name;

        let mut new_manifest = cargo_toml::Manifest::from_str(
            format!(
                "[package]
        name = \"{name}\""
            )
            .as_str(),
        )?;

        // Create new package metadata
        let mut new_package = new_manifest.package.unwrap();
        new_package.description = package.description;
        if workspace_package.version.is_some() {
            new_package.version = Inherited;
        } else {
            new_package.version = package.version;
        }
        if workspace_package.edition.is_none() {
            return Err(Error::InvalidWorkspaceCargoToml(
                "[workspace.package.edition]".to_string(),
            ));
        }
        new_package.edition = Inherited;
        if workspace_package.license.is_some() {
            new_package.license = Some(Inherited);
        }
        if workspace_package.repository.is_some() {
            new_package.repository = Some(Inherited);
        }
        new_package.publish = Set(Publish::Flag(false));

        let mut table = toml::Table::new();
        table.insert("cargo_inherit".to_string(), toml::Value::Boolean(true));
        new_package.metadata = Some(Table(table));

        // Copy over a lib section
        let lib = Product {
            crate_type: vec!["cdylib".to_string()],
            doctest: false,
            ..Default::default()
        };

        new_manifest.lib = Some(lib);

        // TODO: We might want to check rust version here as well, but it's not very trivial.
        // Someone may use a nightly version and we always fail the check because technically the versions aren't the same
        // We could just print a warning if there's a version mismatch

        let mut dependencies = manifest.dependencies;
        let mut new_workspace_dependencies = workspace.dependencies.clone();
        self.inherit_dependencies(printer, &mut new_workspace_dependencies, &mut dependencies)?;
        new_manifest.dependencies = dependencies;

        let mut dev_dependencies = manifest.dev_dependencies;
        self.inherit_dependencies(
            printer,
            &mut new_workspace_dependencies,
            &mut dev_dependencies,
        )?;
        new_manifest.dev_dependencies = dev_dependencies;

        new_manifest.package = Some(new_package);

        let toml_string = toml::to_string_pretty(&new_manifest)?;
        fs::write(example_toml_path, toml_string)?;

        let new_workspace = Workspace {
            dependencies: new_workspace_dependencies,
            ..workspace.clone()
        };
        let new_workspace_manifest = Manifest {
            workspace: Some(new_workspace),
            ..workspace_manifest
        };
        let toml_string = toml::to_string_pretty(&new_workspace_manifest)?;
        fs::write(workspace_toml_path, toml_string)?;

        Ok(())
    }

    fn inherit_dependencies(
        &self,
        printer: &Print,
        workspace_dependencies: &mut DepsSet,
        dependencies: &mut DepsSet,
    ) -> Result<(), Error> {
        let mut new_dependencies = vec![];
        for (dependency_name, example_dep) in dependencies.iter() {
            // This nested if statement gets the major dependency version from the workspace Cargo.toml
            // and from the example Cargo.toml and checks that they are equal.
            // If it fails, it simply prints a warning. But a mismatch is detected,
            // it exits with an error (overridable by --force)
            if let Some(manifest_dep) = workspace_dependencies.get(dependency_name) {
                if let Some(example_major) = Self::try_get_major_version(example_dep)
                    && let Some(manifest_major) = Self::try_get_major_version(manifest_dep)
                {
                    // Check major versions are equal
                    if example_major != manifest_major {
                        if self.force {
                            printer.warnln(format!("Example {dependency_name} dependency version doesn't match manifest version (example might not compile)"));
                        } else {
                            return Err(Error::DependencyVersionMismatch(
                                dependency_name.clone(),
                                example_major,
                                manifest_major,
                            ));
                        }
                    }
                } else {
                    printer.warnln(format!("Workspace or an example Cargo.toml's {dependency_name} dependency version couldn't be parsed, skipping example version validation (if there's a mismatch it might not compile)"));
                }
            } else {
                workspace_dependencies.insert(dependency_name.clone(), example_dep.clone());

                printer.infoln(format!(
                    "Updating workspace Cargo.toml with new dependency {dependency_name}."
                ));
            }

            let mut optional = false;
            let mut features = vec![];

            // Copy details from the example dependency
            if let Dependency::Detailed(detail) = example_dep {
                optional = detail.optional;
                features.clone_from(&detail.features);
            }

            new_dependencies.push((
                dependency_name.clone(),
                Dependency::Inherited(InheritedDependencyDetail {
                    workspace: true,
                    optional,
                    features,
                }),
            ));
        }
        dependencies.extend(new_dependencies);
        Ok(())
    }

    fn try_get_major_version(dependency: &Dependency) -> Option<u32> {
        match dependency {
            Simple(version) => {
                if let Some(Ok(example_version)) = Self::manifest_version_to_major(version) {
                    return Some(example_version);
                }
            }
            Dependency::Inherited(_) => {}
            Dependency::Detailed(detail) => {
                if let Some(version) = &detail.version
                    && let Some(Ok(example_version)) = Self::manifest_version_to_major(version)
                {
                    return Some(example_version);
                }
            }
        }
        None
    }

    fn manifest_version_to_major(manifest_dep: &str) -> Option<Result<u32, ParseIntError>> {
        manifest_dep
            .split('.')
            .next()
            .map(|s| s.chars().filter(char::is_ascii_digit).collect::<String>())
            .map(|s| s.parse::<u32>())
    }

    fn update_workspace_dependencies(
        workspace_path: &Path,
        example_path: &Path,
        tag: &str,
        global_args: &global::Args,
    ) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        let example_cargo_content = fs::read_to_string(example_path.join("Cargo.toml"))?;
        let deps = Self::extract_stellar_dependencies(&example_cargo_content)?;
        if deps.is_empty() {
            return Ok(());
        }

        // Parse the workspace Cargo.toml
        let mut manifest = cargo_toml::Manifest::from_path(workspace_path)?;

        // Ensure workspace.dependencies exists
        if manifest.workspace.is_none() {
            // Create a minimal workspace with just what we need
            let workspace_toml = r"
[workspace]
members = []

[workspace.dependencies]
";
            let workspace: cargo_toml::Workspace<toml::Value> = toml::from_str(workspace_toml)?;
            manifest.workspace = Some(workspace);
        }
        let workspace = manifest.workspace.as_mut().unwrap();

        let mut workspace_deps = workspace.dependencies.clone();

        let mut added_deps = Vec::new();
        let mut updated_deps = Vec::new();

        for dep in deps {
            let git_dep = cargo_toml::DependencyDetail {
                git: Some("https://github.com/OpenZeppelin/stellar-contracts".to_string()),
                tag: Some(tag.to_string()),
                ..Default::default()
            };

            if let Some(existing_dep) = workspace_deps.clone().get(&dep) {
                // Check if we need to update the tag
                if let cargo_toml::Dependency::Detailed(detail) = existing_dep
                    && let Some(existing_tag) = &detail.tag
                    && existing_tag != tag
                {
                    workspace_deps.insert(
                        dep.clone(),
                        cargo_toml::Dependency::Detailed(Box::new(git_dep)),
                    );
                    updated_deps.push((dep, existing_tag.clone()));
                }
            } else {
                workspace_deps.insert(
                    dep.clone(),
                    cargo_toml::Dependency::Detailed(Box::new(git_dep)),
                );
                added_deps.push(dep);
            }
        }

        if !added_deps.is_empty() || !updated_deps.is_empty() {
            workspace.dependencies = workspace_deps;
            // Write the updated manifest back to file
            let toml_string = toml::to_string_pretty(&manifest)?;
            fs::write(workspace_path, toml_string)?;

            if !added_deps.is_empty() {
                printer.infoln("Added the following dependencies to workspace:");
                for dep in added_deps {
                    printer.println(format!("   • {dep}"));
                }
            }

            if !updated_deps.is_empty() {
                printer.infoln("Updated the following dependencies:");
                for (dep, old_tag) in updated_deps {
                    printer.println(format!("   • {dep}: {old_tag} -> {tag}"));
                }
            }
        }

        Ok(())
    }

    fn extract_stellar_dependencies(cargo_toml_content: &str) -> Result<Vec<String>, Error> {
        let manifest: cargo_toml::Manifest = toml::from_str(cargo_toml_content)?;

        Ok(manifest
            .dependencies
            .iter()
            .filter(|(dep_name, _)| dep_name.starts_with("stellar-"))
            .filter_map(|(dep_name, dep_detail)| match dep_detail {
                cargo_toml::Dependency::Detailed(detail)
                    if !(detail.inherited || detail.git.is_some()) =>
                {
                    None
                }
                _ => Some(dep_name.clone()),
            })
            .collect())
    }

    fn examples_list(examples_path: PathBuf) -> Result<Vec<String>, Error> {
        let mut examples: Vec<String> = if examples_path.exists() {
            fs::read_dir(examples_path)?
                .filter_map(std::result::Result::ok)
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .map(std::string::ToString::to_string)
                })
                .collect()
        } else {
            Vec::new()
        };

        examples.sort();

        Ok(examples)
    }

    async fn list_examples(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        let examples_info = self.ensure_cache_updated(&printer).await?;

        printer.infoln("Fetching available contract examples...");

        let oz_examples_path = examples_info.oz_examples_path.join("examples");

        let oz_examples = Self::examples_list(oz_examples_path)?;
        let soroban_examples = Self::examples_list(examples_info.soroban_examples_path)?;

        printer.println("\nAvailable contract examples:");
        printer.println("────────────────────────────────");
        printer.println(format!("From {SOROBAN_EXAMPLES_REPO}:"));

        for example in &soroban_examples {
            printer.println(format!("  📁 {STELLAR_PREFIX}{example}"));
        }

        printer.println("────────────────────────────────");
        printer.println(format!("From {OZ_EXAMPLES_REPO}"));

        for example in &oz_examples {
            printer.println(format!("  📁 {OZ_PREFIX}{example}"));
        }

        printer.println("\nUsage:");
        printer.println("   stellar-scaffold contract generate --from <example-name>");
        printer.println(
            "   Example (soroban-examples): stellar-scaffold contract generate --from stellar/hello-world",
        );
        printer.println("   Example (OpenZeppelin examples): stellar-scaffold contract generate --from oz/nft-royalties");

        Ok(())
    }

    async fn fetch_latest_oz_release() -> Result<Release, Error> {
        Self::fetch_latest_release_from_url(&format!(
            "https://api.github.com/repos/OpenZeppelin/stellar-contracts/releases/tags/{LATEST_SUPPORTED_OZ_RELEASE}",
        ))
        .await
    }

    async fn fetch_latest_soroban_examples_release() -> Result<Release, Error> {
        Self::fetch_latest_release_from_url(
            "https://api.github.com/repos/stellar/soroban-examples/releases/latest",
        )
        .await
    }

    async fn fetch_latest_release_from_url(url: &str) -> Result<Release, Error> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "stellar-scaffold-cli")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Reqwest(response.error_for_status().unwrap_err()));
        }

        let release: Release = response.json().await?;
        Ok(release)
    }

    async fn cache_oz_repository(repo_cache_path: &Path, tag_name: &str) -> Result<(), Error> {
        Self::cache_repository("OpenZeppelin/stellar-contracts", repo_cache_path, tag_name).await
    }

    async fn cache_soroban_examples_repository(
        repo_cache_path: &Path,
        tag_name: &str,
    ) -> Result<(), Error> {
        Self::cache_repository("stellar/soroban-examples", repo_cache_path, tag_name).await
    }

    fn filter_soroban_examples_repository(repo_cache_path: &Path) -> Result<(), Error> {
        // Atomic multiswap imports atomic swap contract which is currently not supported
        let ignore_list = HashSet::from(["workspace", "atomic_multiswap"]);
        let rd = repo_cache_path.read_dir()?;
        for path in rd {
            let path = path?.path();
            if !path.is_dir() {
                fs::remove_file(path)?;
            } else if path.is_dir() {
                // Remove ignored files and directories
                if let Some(path_file_name) = path.file_name()
                    && let Some(path_file_name) = path_file_name.to_str()
                    && ignore_list.contains(path_file_name)
                {
                    fs::remove_dir_all(path)?;
                    continue;
                }

                // Remove hidden directories (e.g. .git)
                if path.starts_with(".") {
                    fs::remove_dir_all(path)?;
                } else {
                    // Only allow simple examples for now (where Cargo.toml exists in the root)
                    let rd = path.read_dir()?;
                    let mut is_simple_example = false;
                    for entry in rd {
                        let entry = entry?;
                        if entry.path().is_file() && entry.file_name() == "Cargo.toml" {
                            is_simple_example = true;
                        }
                    }
                    if !is_simple_example {
                        fs::remove_dir_all(path)?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn cache_repository(
        repo: &str,
        repo_cache_path: &Path,
        tag_name: &str,
    ) -> Result<(), Error> {
        // Download and extract the specific tag directly
        Self::download_and_extract_tag(repo, repo_cache_path, tag_name).await?;

        if repo_cache_path.read_dir()?.next().is_none() {
            return Err(Error::GitCloneFailed(format!(
                "Failed to download repository release {tag_name} to cache"
            )));
        }

        Ok(())
    }

    async fn download_and_extract_tag(
        repo: &str,
        dest_path: &Path,
        tag_name: &str,
    ) -> Result<(), Error> {
        let url = format!("https://github.com/{repo}/archive/{tag_name}.tar.gz",);

        // Download the tar.gz file
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", "stellar-scaffold-cli")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::GitCloneFailed(format!(
                "Failed to download release {tag_name} from {url}: HTTP {}",
                response.status()
            )));
        }

        // Get the response bytes
        let bytes = response.bytes().await?;

        fs::create_dir_all(dest_path)?;

        // Extract the tar.gz in a blocking task to avoid blocking the async runtime
        let dest_path = dest_path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let tar = GzDecoder::new(std::io::Cursor::new(bytes));
            let mut archive = Archive::new(tar);

            for entry in archive.entries()? {
                let mut entry = entry?;
                let path = entry.path()?;

                // Strip the root directory (stellar-contracts-{tag}/)
                let stripped_path = path.components().skip(1).collect::<std::path::PathBuf>();

                if stripped_path.as_os_str().is_empty() {
                    continue;
                }

                let dest_file_path = dest_path.join(&stripped_path);

                if entry.header().entry_type().is_dir() {
                    std::fs::create_dir_all(&dest_file_path)?;
                } else {
                    if let Some(parent) = dest_file_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    entry.unpack(&dest_file_path)?;
                }
            }

            Ok::<(), std::io::Error>(())
        })
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?
        .map_err(Error::Io)?;

        Ok(())
    }

    async fn ensure_cache_updated(&self, printer: &Print) -> Result<ExamplesInfo, Error> {
        let cache_dir = dirs::cache_dir().ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cache directory not found",
            ))
        })?;

        let cli_cache_path = cache_dir.join("stellar-scaffold-cli");

        let oz_cache_path = cli_cache_path.join("openzeppelin-stellar-contracts");
        let soroban_examples_cache_path = cli_cache_path.join("soroban_examples");

        Self::update_cache(&oz_cache_path, &soroban_examples_cache_path)
            .await
            .or_else(|e| {
                printer.warnln(format!("Failed to update examples cache: {e}"));
                Self::get_latest_known_examples(&oz_cache_path, &soroban_examples_cache_path)
            })
    }

    async fn update_cache(
        oz_cache_path: &Path,
        soroban_examples_cache_path: &Path,
    ) -> Result<ExamplesInfo, Error> {
        // Get the latest release tag
        let Release { tag_name } = Self::fetch_latest_oz_release().await?;
        let oz_repo_cache_path = oz_cache_path.join(&tag_name);
        if !oz_repo_cache_path.exists() {
            Self::cache_oz_repository(&oz_repo_cache_path, &tag_name).await?;
        }
        let oz_tag_name = tag_name;

        let Release { tag_name } = Self::fetch_latest_soroban_examples_release().await?;
        let soroban_examples_cache_path = soroban_examples_cache_path.join(&tag_name);
        if !soroban_examples_cache_path.exists() {
            Self::cache_soroban_examples_repository(&soroban_examples_cache_path, &tag_name)
                .await?;
            Self::filter_soroban_examples_repository(&soroban_examples_cache_path)?;
        }

        Ok(ExamplesInfo {
            oz_examples_path: oz_repo_cache_path,
            oz_version_tag: oz_tag_name,
            soroban_examples_path: soroban_examples_cache_path,
            soroban_version_tag: tag_name,
        })
    }

    fn get_latest_known_examples(
        oz_cache_path: &Path,
        soroban_examples_cache_path: &Path,
    ) -> Result<ExamplesInfo, Error> {
        if oz_cache_path.exists() && soroban_examples_cache_path.exists() {
            let oz_tag_name = Self::get_latest_known_tag(oz_cache_path)?;
            let soroban_examples_tag_name =
                Self::get_latest_known_tag(soroban_examples_cache_path)?;

            let oz_repo_cache_path = oz_cache_path.join(&oz_tag_name);
            let soroban_examples_cache_path =
                soroban_examples_cache_path.join(&soroban_examples_tag_name);

            Ok(ExamplesInfo {
                oz_examples_path: oz_repo_cache_path,
                oz_version_tag: oz_tag_name,
                soroban_examples_path: soroban_examples_cache_path,
                soroban_version_tag: soroban_examples_tag_name,
            })
        } else {
            Err(Error::UpdateExamplesCache)
        }
    }

    fn get_latest_known_tag(example_cache_path: &Path) -> Result<String, Error> {
        let rd = example_cache_path.read_dir()?;
        let max_tag = rd
            .filter_map(Result::ok)
            .filter(|x| x.path().is_dir())
            .filter_map(|x| x.file_name().to_str().map(ToString::to_string))
            .max();
        max_tag.ok_or(Error::UpdateExamplesCache)
    }

    fn copy_directory_contents(source: &Path, dest: &Path) -> Result<(), Error> {
        let copy_options = fs_extra::dir::CopyOptions::new()
            .overwrite(true)
            .content_only(true);

        fs_extra::dir::copy(source, dest, &copy_options)
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;

        Ok(())
    }

    fn get_workspace_root(path: &Path) -> Result<PathBuf, Error> {
        let output = Command::new("cargo")
            .arg("locate-project")
            .arg("--workspace")
            .arg("--message-format")
            .arg("json")
            .arg("--manifest-path")
            .arg(path)
            .output()?;

        if !output.status.success() {
            return Err(Error::CargoError);
        }

        let json_str = String::from_utf8(output.stdout).map_err(|_| Error::CargoError)?;
        let parsed_json: Value = serde_json::from_str(&json_str).map_err(|_| Error::CargoError)?;

        let workspace_root_str = parsed_json["root"].as_str().ok_or(Error::CargoError)?;

        Ok(PathBuf::from(workspace_root_str))
    }

    fn output_dir(&self, example_name: &str) -> PathBuf {
        PathBuf::from("contracts").join(
            self.output
                .as_deref()
                .unwrap_or_else(|| Path::new(example_name)),
        )
    }
}

struct ExamplesInfo {
    oz_examples_path: PathBuf,
    oz_version_tag: String,
    soroban_examples_path: PathBuf,
    #[allow(dead_code)] // TODO: remove if not used
    soroban_version_tag: String,
}

fn open_wizard(global_args: &global::Args) -> Result<(), Error> {
    let printer = Print::new(global_args.quiet);

    printer.infoln("Opening OpenZeppelin Contract Wizard...");

    let url = "https://wizard.openzeppelin.com/stellar";

    webbrowser::open(url)
        .map_err(|e| Error::BrowserFailed(format!("Failed to open browser: {e}")))?;

    printer.checkln("Opened Contract Wizard in your default browser");
    printer.println("\nInstructions:");
    printer.println("   1. Configure your contract in the wizard");
    printer.println("   2. Click 'Download' to get your contract files");
    printer.println("   3. Extract the downloaded ZIP file");
    printer.println("   4. Move the contract folder to your contracts/ directory");
    printer.println("   5. Add the contract to your workspace Cargo.toml if needed");
    printer.println(
        "   6. You may need to modify your environments.toml file to add constructor arguments",
    );
    printer.infoln(
        "The wizard will generate a complete Soroban contract with your selected features!",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url};

    fn create_test_cmd(from: Option<String>, ls: bool, from_wizard: bool) -> Cmd {
        Cmd {
            from,
            ls,
            from_wizard,
            output: None,
            force: false,
        }
    }

    #[tokio::test]
    #[ignore = "requires additional setup beyond HTTP mock"]
    async fn test_ls_command() {
        let cmd = create_test_cmd(None, true, false);
        let global_args = global::Args::default();

        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/contents/examples",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"name": "example1", "type": "dir"}, {"name": "example2", "type": "dir"}]"#)
        .create();

        let result = cmd.run(&global_args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_latest_release() {
        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/releases/latest",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "tag_name": "v1.2.3",
                "name": "Release v1.2.3",
                "published_at": "2024-01-15T10:30:00Z"
            }"#,
        )
        .create();

        let mock_url = format!(
            "{}/repos/OpenZeppelin/stellar-contracts/releases/latest",
            server_url()
        );
        let result = Cmd::fetch_latest_release_from_url(&mock_url).await;

        assert!(result.is_ok());
        let release = result.unwrap();
        assert_eq!(release.tag_name, "v1.2.3");
    }

    #[tokio::test]
    async fn test_fetch_latest_release_error() {
        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/releases/latest",
        )
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "Not Found"}"#)
        .create();

        let mock_url = format!(
            "{}/repos/OpenZeppelin/stellar-contracts/releases/latest",
            server_url()
        );
        let result = Cmd::fetch_latest_release_from_url(&mock_url).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_action_specified() {
        let cmd = create_test_cmd(None, false, false);
        let global_args = global::Args::default();
        let result = cmd.run(&global_args).await;
        assert!(matches!(result, Err(Error::NoActionSpecified)));
    }
}
