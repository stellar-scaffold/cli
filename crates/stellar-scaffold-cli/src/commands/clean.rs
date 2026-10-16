use crate::config::{self, schema::Project};
use crate::state::{self, State};
use cargo_metadata::Metadata;
use clap::Parser;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};
use stellar_cli::{commands::global, config::locator, print::Print};
/// A command to clean the scaffold-generated artifacts from a project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Path to Cargo.toml
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] io::Error),
}

// cleans up scaffold artifacts
// - target/stellar ✅
// - clients_dir/* — Binding packages + flattened Clients index.ts (but not
//   checked-into-git files like .gitkeep) ✅
// - contract aliases recorded in .scaffold/state.json, and the file itself ✅
// - identities are kept: they're machine-wide, shared with other projects

// - should this be deleting target/stellar/local and target/stellar/testnet specifically to avoid deleting mainnet?
// - what about target/packages?
// - what about target/wasm32v1-none/release/guess_the_number.wasm

impl Cmd {
    pub fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);
        printer.infoln("Starting workspace cleanup");

        let cargo_meta = match &self.manifest_path {
            Some(manifest_path) => cargo_metadata::MetadataCommand::new()
                .manifest_path(manifest_path)
                .no_deps()
                .exec()
                .unwrap(),
            _ => cargo_metadata::MetadataCommand::new()
                .no_deps()
                .exec()
                .unwrap(),
        };

        Self::clean_target_stellar(&cargo_meta, &printer)?;

        let workspace_root: PathBuf = cargo_meta.workspace_root.into();
        let project = config::project(&workspace_root);

        Self::clean_clients(&workspace_root, &project, &printer)?;

        Self::clean_state(&workspace_root, &global_args.locator, &printer)?;

        Ok(())
    }

    fn clean_target_stellar(cargo_meta: &Metadata, printer: &Print) -> Result<(), Error> {
        let target_dir = &cargo_meta.target_directory;
        let stellar_dir = target_dir.join("stellar");
        if stellar_dir.exists() {
            fs::remove_dir_all(&stellar_dir)?;
        } else {
            printer.infoln(format!(
                "Skipping target clean: {stellar_dir} does not exist"
            ));
        }
        Ok(())
    }

    /// Clean the generated contract layer (`clients_dir`): both the Binding
    /// packages and the flattened Clients `index.ts` live here. Git-tracked
    /// entries and `.gitkeep` are preserved.
    fn clean_clients(
        workspace_root: &Path,
        project: &Project,
        printer: &Print,
    ) -> Result<(), Error> {
        let clients_path = workspace_root.join(&project.clients_dir);
        let clients_dir_str = project
            .clients_dir
            .to_str()
            .unwrap_or("app-lib/clients")
            .to_string();
        let git_tracked_entries = Self::git_tracked_entries(workspace_root, &clients_dir_str);
        Self::clean_dir(workspace_root, &clients_path, &git_tracked_entries, printer)
    }

    /// Remove the stellar-cli aliases this checkout's builds saved, then its
    /// `.scaffold/state.json`, so the next build deploys fresh. An alias that
    /// has since been pointed elsewhere isn't Scaffold's any more and is kept.
    /// Keys are kept too: they're shared by every project on this machine.
    fn clean_state(
        workspace_root: &Path,
        locator: &locator::Args,
        printer: &Print,
    ) -> Result<(), Error> {
        let state = State::load(workspace_root).unwrap_or_else(|e| {
            printer.warnln(format!(
                "Can't read {}, so its aliases are kept: {e}",
                State::path(workspace_root).display()
            ));
            State::default()
        });
        for (passphrase, alias, contract_id) in state.aliases() {
            let target = locator.get_contract_id(alias, passphrase).ok().flatten();
            if target.is_none_or(|c| format!("{c}") != contract_id) {
                continue;
            }
            match locator.remove_contract_id(passphrase, alias) {
                Ok(()) => printer.infoln(format!("Removed contract alias: {alias}")),
                Err(e) => printer.warnln(format!("Failed to remove contract alias {alias}: {e}")),
            }
        }
        if State::remove(workspace_root)? {
            printer.infoln(format!("Removed {}/{}", state::DIR, state::FILE));
        }
        Ok(())
    }

    fn git_tracked_entries(workspace_root: &Path, subdir: &str) -> Vec<String> {
        let output = Command::new("git")
            .args(["ls-files", subdir])
            .current_dir(workspace_root)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout
                    .lines()
                    .map(std::string::ToString::to_string)
                    .collect()
            }
            _ => {
                // If git command fails, return empty list (no files will be preserved)
                Vec::new()
            }
        }
    }

    // cleans the given directory while preserving git-tracked files and .gitkeep
    // (everything else in the generated clients_dir like app-lib/clients/ is
    // regenerable). Authored files are kept by virtue of being git-tracked.
    fn clean_dir(
        workspace_root: &Path,
        dir_to_clean: &Path,
        git_tracked_entries: &[String],
        printer: &Print,
    ) -> Result<(), Error> {
        if dir_to_clean.exists() {
            for entry in fs::read_dir(dir_to_clean)? {
                let entry = entry?;
                let path = entry.path();
                let relative_path = path.strip_prefix(workspace_root).unwrap_or(&path);
                let relative_str = relative_path.to_string_lossy().replace('\\', "/");

                // Skip if this is a git-tracked file
                if git_tracked_entries.contains(&relative_str) {
                    continue;
                }

                // Preserve .gitkeep regardless of git status
                let filename = path.file_name().and_then(|n| n.to_str());
                if filename == Some(".gitkeep") {
                    continue;
                }

                // Remove the file or directory
                if path.is_dir() {
                    fs::remove_dir_all(&path).unwrap();
                } else {
                    fs::remove_file(&path).unwrap();
                }
                printer.infoln(format!("Removed {relative_str}"));
            }
        } else {
            printer.infoln(format!(
                "Skipping clean: {} does not exist",
                dir_to_clean.display()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_workspace(temp_dir: &Path) -> PathBuf {
        let manifest_path = temp_dir.join("Cargo.toml");
        fs::write(
            &manifest_path,
            r#"[package]
name = "soroban-hello-world-contract"
version = "0.0.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]
"#,
        )
        .unwrap();

        let src_dir = temp_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "// dummy lib").unwrap();

        manifest_path
    }

    #[test]
    fn clean_state_removes_only_aliases_it_still_owns() {
        const LOCAL: &str = "Standalone Network ; February 2017";
        let workspace = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();
        let locator = locator::Args {
            config_dir: Some(config_dir.path().to_path_buf()),
        };
        let ours = stellar_strkey::Contract([1; 32]);
        let theirs = stellar_strkey::Contract([2; 32]);

        let mut state = State::default();
        for (name, alias) in [("counter", "app-counter"), ("token", "app-token")] {
            let contract = state::ContractState {
                contract_id: format!("{ours}"),
                alias: Some(alias.to_string()),
            };
            state.record("local", LOCAL, name, contract);
        }
        state.save(workspace.path()).unwrap();
        locator
            .save_contract_id(LOCAL, &ours, "app-counter")
            .unwrap();
        // pointed elsewhere since the build recorded it
        locator
            .save_contract_id(LOCAL, &theirs, "app-token")
            .unwrap();

        Cmd::clean_state(workspace.path(), &locator, &Print::new(true)).unwrap();

        assert_eq!(locator.get_contract_id("app-counter", LOCAL).unwrap(), None);
        assert_eq!(
            locator.get_contract_id("app-token", LOCAL).unwrap(),
            Some(theirs)
        );
        assert!(!State::path(workspace.path()).exists());
    }

    #[test]
    fn test_clean_target_stellar() {
        let global_args = global::Args::default();
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = create_test_workspace(temp_dir.path());

        let target_stellar_path = temp_dir.path().join("target").join("stellar");
        std::fs::create_dir_all(&target_stellar_path).unwrap();

        let cmd = Cmd {
            manifest_path: Some(manifest_path),
        };
        assert!(cmd.run(&global_args).is_ok());

        assert!(
            !target_stellar_path.exists(),
            "target/stellar should be removed"
        );
    }

    #[test]
    fn test_clean_binding_packages() {
        let global_args = global::Args::default();
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = create_test_workspace(temp_dir.path());

        // Default clients_dir is `app-lib/clients` — Binding packages live here.
        let clients_path = temp_dir.path().join("app-lib").join("clients");
        let test_package_path = clients_path.join("test_contract_package");
        std::fs::create_dir_all(&test_package_path).unwrap();

        let gitkeep_path = clients_path.join(".gitkeep");
        fs::write(&gitkeep_path, "").unwrap();

        let cmd = Cmd {
            manifest_path: Some(manifest_path),
        };

        assert!(cmd.run(&global_args).is_ok());

        assert!(
            !test_package_path.exists(),
            "app-lib/clients/test_contract_package/ should be removed"
        );
        assert!(
            gitkeep_path.exists(),
            "app-lib/clients/.gitkeep should be preserved"
        );
    }

    #[test]
    fn test_clean_clients_index() {
        let global_args = global::Args::default();
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = create_test_workspace(temp_dir.path());

        // Default clients_dir is `app-lib/clients` — the flattened index.ts lives here.
        let clients_path = temp_dir.path().join("app-lib").join("clients");
        std::fs::create_dir_all(&clients_path).unwrap();

        let index_path = clients_path.join("index.ts");
        fs::write(&index_path, "").unwrap();

        let gitkeep_path = clients_path.join(".gitkeep");
        fs::write(&gitkeep_path, "").unwrap();

        let cmd = Cmd {
            manifest_path: Some(manifest_path),
        };

        assert!(cmd.run(&global_args).is_ok());

        assert!(
            !index_path.exists(),
            "app-lib/clients/index.ts (generated) should be removed"
        );
        assert!(
            gitkeep_path.exists(),
            "app-lib/clients/.gitkeep should be preserved"
        );
    }
}
