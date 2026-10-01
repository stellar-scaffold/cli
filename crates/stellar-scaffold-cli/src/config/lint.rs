//! Checks on individual values of a parsed [`Config`]: names, `source`
//! values, crates, scripts, method names, and `${…}` syntax. Rules that
//! cross-reference networks and contracts live in `resolve.rs`.

use std::path::{Path, PathBuf};

use serde_saphyr::Spanned;

use super::diagnostic::{Code, Diagnostic, span, suggest};
use super::interpolate::{self, Namespace};
use super::is_valid_name;
use super::schema::{Config, ContractSettings, Project, SpannedMap, Value};
use super::source::{self, SourceType};

/// Everything the checks need from outside the file.
pub struct Env<'a> {
    pub root: &'a Path,
    /// Package names and directories of the workspace's `cdylib` crates, or
    /// `None` if the cargo workspace could not be read.
    pub crates: Option<&'a [(String, PathBuf)]>,
}

pub fn lint(config: &Config, env: &Env) -> Vec<Diagnostic> {
    let mut l = Lint {
        env,
        project: &config.project,
        diags: Vec::new(),
    };
    for (key, net) in config.networks.iter() {
        l.name(key, "network");
        if let Some(e) = &net.extends {
            l.name(e, "network");
        }
        for account in net.accounts.iter().flatten().chain(&net.default_account) {
            l.name(account, "account");
        }
        let env_only = &[Namespace::Env];
        for t in net.rpc_url.iter().chain(&net.network_passphrase) {
            l.template(t, env_only);
        }
        for (_, v) in net.rpc_headers.iter() {
            l.template(v, env_only);
        }
    }
    for (key, contract) in config.contracts.iter() {
        if l.contract_name(key) {
            l.settings(&format!("contract `{}`", key.value), contract);
            if let Some(networks) = &contract.networks {
                for (net_key, settings) in networks.value.iter() {
                    let what = format!("contract `{}` on network `{}`", key.value, net_key.value);
                    if let Some(nested) = &settings.networks {
                        l.diags.push(
                            Diagnostic::error(
                                Code::UnknownKey,
                                format!("{what} cannot have its own `networks:`"),
                            )
                            .at(span(nested)),
                        );
                    }
                    l.settings(&what, settings);
                }
            }
        }
    }
    l.diags
}

struct Lint<'a> {
    env: &'a Env<'a>,
    project: &'a Project,
    diags: Vec<Diagnostic>,
}

impl Lint<'_> {
    fn name(&mut self, s: &Spanned<String>, what: &str) {
        if !is_valid_name(&s.value) {
            self.diags.push(
                Diagnostic::error(
                    Code::InvalidName,
                    format!(
                        "`{}` is not a valid {what} name (letters, digits, `-`, `_`)",
                        s.value
                    ),
                )
                .at(span(s)),
            );
        }
    }

    /// Contract names become npm package names and camelCase exports, so they
    /// are held to npm's rules: lowercase, starting with a letter.
    fn contract_name(&mut self, key: &Spanned<String>) -> bool {
        let valid = key.value.starts_with(|c: char| c.is_ascii_lowercase())
            && key
                .value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
        if !valid {
            self.diags.push(
                Diagnostic::error(
                    Code::InvalidName,
                    format!(
                        "`{}` is not a valid contract name (lowercase letter first, then lowercase letters, digits, `-`, `_`)",
                        key.value
                    ),
                )
                .at(span(key)),
            );
        }
        valid
    }

    /// Check `${…}` syntax, and that only `allowed` namespaces are used.
    fn template(&mut self, s: &Spanned<String>, allowed: &[Namespace]) {
        match interpolate::parse(&s.value) {
            Ok(segments) => {
                if let Some(ns) =
                    interpolate::namespaces(&segments).find(|ns| !allowed.contains(ns))
                {
                    let ns = format!("{ns:?}").to_lowercase();
                    self.diags.push(
                        Diagnostic::error(
                            Code::Interpolation,
                            format!("`${{{ns}.…}}` cannot be used here"),
                        )
                        .at(span(s)),
                    );
                }
            }
            Err(e) => self
                .diags
                .push(Diagnostic::error(Code::Interpolation, e).at(span(s))),
        }
    }

    fn settings(&mut self, what: &str, s: &ContractSettings) {
        match (&s.ty, &s.source) {
            (Some(ty), Some(src)) => self.source(ty.value, src),
            (Some(_), None) | (None, Some(_)) => {
                let at =
                    s.ty.as_ref()
                        .map(span)
                        .or(s.source.as_ref().map(span))
                        .unwrap_or_default();
                self.diags.push(
                    Diagnostic::error(
                        Code::MissingSource,
                        format!("{what} sets only one of `type` and `source`"),
                    )
                    .at(at)
                    .help("set `type` and `source` together at the same level"),
                );
            }
            (None, None) => {}
        }
        if let Some(name) = &s.from_network {
            self.name(name, "network");
        }
        if let Some(name) = &s.signer {
            self.name(name, "account");
        }
        if let Some(args) = &s.args {
            self.args(&args.value);
        }
        if let Some(methods) = &s.after_deploy {
            for m in &methods.value {
                self.method(m);
            }
        }
        if let Some(script) = &s.after_deploy_script {
            self.script(script);
        }
    }

    fn source(&mut self, ty: SourceType, src: &Spanned<String>) {
        match source::check(ty, &src.value, self.env.root) {
            source::Check::Ok => {}
            source::Check::Invalid(e) => {
                self.diags
                    .push(Diagnostic::error(Code::InvalidSource, e).at(span(src)));
            }
            source::Check::Unpinned => self.diags.push(
                Diagnostic::warning(
                    Code::UnpinnedVersion,
                    format!("`{}` has no pinned version", src.value),
                )
                .at(span(src))
                .help("append `@<version>` so a new release can't change what you build against"),
            ),
            source::Check::MissingFile => self.diags.push(
                Diagnostic::warning(
                    Code::WasmFileMissing,
                    format!("`{}` does not exist yet", src.value),
                )
                .at(span(src))
                .help("fine if a build step produces it; otherwise check the path"),
            ),
        }
        if ty == SourceType::Workspace {
            self.crate_exists(src);
        }
    }

    fn crate_exists(&mut self, src: &Spanned<String>) {
        let Some(all) = self.env.crates else { return };
        let contracts_dir = self.env.root.join(&self.project.contracts_dir);
        let crates: Vec<&str> = all
            .iter()
            .filter(|(_, dir)| dir.starts_with(&contracts_dir))
            .map(|(name, _)| name.as_str())
            .collect();
        if crates.contains(&src.value.as_str()) {
            return;
        }
        let dir = self.project.contracts_dir.display();
        let mut d = Diagnostic::error(
            Code::CrateNotFound,
            format!("no contract crate named `{}` in `{dir}`", src.value),
        )
        .at(span(src));
        if let Some(s) = suggest(&src.value, crates.iter().copied()) {
            d = d.help(format!("did you mean `{s}`?"));
        } else if !crates.is_empty() {
            d = d.help(format!("found: {}", crates.join(", ")));
        }
        self.diags.push(d);
    }

    fn args(&mut self, args: &SpannedMap<Spanned<Value>>) {
        for (_, v) in args.iter() {
            self.arg_strings(v);
        }
    }

    fn arg_strings(&mut self, v: &Spanned<Value>) {
        let allowed = &[Namespace::Account, Namespace::Env, Namespace::Network];
        match &v.value {
            Value::Str(s) => {
                let s = Spanned::new(s.clone(), v.referenced, v.defined);
                self.template(&s, allowed);
            }
            Value::Seq(items) => items.iter().for_each(|i| self.arg_strings(i)),
            Value::Map(map) => map.iter().for_each(|(_, i)| self.arg_strings(i)),
            _ => {}
        }
    }

    fn method(&mut self, m: &Spanned<String>) {
        let valid = m
            .value
            .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && m.value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid {
            self.diags.push(
                Diagnostic::error(Code::InvalidName, format!("`{}` is not a contract method name", m.value))
                    .at(span(m))
                    .help("`after-deploy` takes method names only; for calls with arguments use `after-deploy-script`"),
            );
        }
    }

    fn script(&mut self, s: &Spanned<String>) {
        let path = self.env.root.join(&s.value);
        if !path.is_file() {
            self.diags.push(
                Diagnostic::error(
                    Code::AfterDeployScript,
                    format!("script `{}` does not exist", s.value),
                )
                .at(span(s)),
            );
        } else if !is_executable(&path) {
            self.diags.push(
                Diagnostic::error(
                    Code::AfterDeployScript,
                    format!("script `{}` is not executable", s.value),
                )
                .at(span(s))
                .help(format!("run `chmod +x {}` and add a shebang line", s.value)),
            );
        }
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true
}
