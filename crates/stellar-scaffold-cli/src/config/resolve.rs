//! Resolution of `extends` chains and per-network contract overrides, plus
//! the validation rules that need a resolved view.
//!
//! Merge rules, used for both `extends` and contract defaults → per-network
//! settings: scalars override, maps merge, lists replace (never append).

use std::collections::HashMap;

use serde_saphyr::Spanned;
use stellar_cli::config::network::{DEFAULTS, passphrase};

use super::diagnostic::{Code, Diagnostic, Span, span, suggest};
use super::interpolate::{self, Segment};
use super::schema::{Config, ContractSettings, NetworkDef, SpannedMap, Value};
use super::source::SourceType;

/// Longest allowed `extends` chain, counted in hops.
const MAX_EXTENDS_DEPTH: usize = 3;

/// A string that may contain `${…}` references. Syntax errors are reported
/// by `lint`; here they degrade to the literal text.
#[derive(Debug, Clone)]
pub struct Template {
    pub raw: String,
    pub segments: Vec<Segment>,
}

impl Template {
    pub fn new(raw: &str) -> Self {
        Self {
            raw: raw.to_string(),
            segments: interpolate::parse(raw)
                .unwrap_or_else(|_| vec![Segment::Lit(raw.to_string())]),
        }
    }

    fn is_literal(&self) -> bool {
        self.segments.iter().all(|s| matches!(s, Segment::Lit(_)))
    }
}

/// A network with `extends` and built-in defaults applied.
#[derive(Debug, Clone)]
pub struct Network {
    pub name: String,
    /// Ancestors from nearest to furthest, e.g. `[testnet]` for `preview`.
    pub extends: Vec<String>,
    pub rpc_url: Option<Template>,
    pub passphrase: Option<Template>,
    pub rpc_headers: Vec<(String, Template)>,
    pub accounts: Vec<String>,
    /// Explicit `default-account`, possibly inherited.
    pub explicit_default_account: Option<String>,
    pub explicit_start_container: Option<bool>,
    pub explicit_allow_deploy: Option<bool>,
}

impl Network {
    fn empty(name: &str) -> Self {
        Self {
            name: name.to_string(),
            extends: Vec::new(),
            rpc_url: None,
            passphrase: None,
            rpc_headers: Vec::new(),
            accounts: Vec::new(),
            explicit_default_account: None,
            explicit_start_container: None,
            explicit_allow_deploy: None,
        }
    }

    /// Built-in names start from stellar-cli's URL and passphrase.
    fn builtin(name: &str) -> Self {
        let mut net = Self::empty(name);
        if let Some((rpc, pass)) = DEFAULTS.get(name) {
            // stellar-cli's mainnet entry is a placeholder, not a usable URL.
            net.rpc_url = (!rpc.starts_with("Bring Your Own")).then(|| Template::new(rpc));
            net.passphrase = Some(Template::new(pass));
        }
        net
    }

    /// The passphrase when it is a literal. `None` if unset or if it depends
    /// on the environment, in which case passphrase-based rules are skipped.
    pub fn literal_passphrase(&self) -> Option<&str> {
        self.passphrase
            .as_ref()
            .filter(|t| t.is_literal())
            .map(|t| t.raw.as_str())
    }

    fn is_public(&self) -> bool {
        self.literal_passphrase().is_some_and(|p| {
            [
                passphrase::TESTNET,
                passphrase::FUTURENET,
                passphrase::MAINNET,
            ]
            .contains(&p)
        })
    }

    fn is_local(&self) -> bool {
        self.literal_passphrase() == Some(passphrase::LOCAL)
    }

    /// Only one account: it signs. Otherwise the explicit `default-account`,
    /// falling back to the first listed.
    pub fn default_account(&self) -> Option<&str> {
        if self.accounts.len() == 1 {
            return self.accounts.first().map(String::as_str);
        }
        self.explicit_default_account
            .as_deref()
            .or_else(|| self.accounts.first().map(String::as_str))
    }

    /// Explicit value, else true only for the local (standalone) passphrase,
    /// so custom local networks work without being named `local`.
    pub fn start_container(&self) -> bool {
        self.explicit_start_container
            .unwrap_or_else(|| self.is_local())
    }

    /// Explicit value, else false for public passphrases. Derived from the
    /// resolved passphrase, never the network's name, so custom networks need
    /// no special cases.
    pub fn allow_deploy(&self) -> bool {
        self.explicit_allow_deploy
            .unwrap_or_else(|| !self.is_public())
    }
}

/// A contract's effective settings on one network.
#[derive(Debug, Clone)]
pub struct Contract {
    pub name: String,
    pub ty: SourceType,
    pub source: String,
    pub from_network: Option<String>,
    /// The signer, falling back to the network's default account.
    pub signer: Option<String>,
    pub args: SpannedMap<Spanned<Value>>,
    pub after_deploy: Vec<String>,
    pub after_deploy_script: Option<String>,
    /// Whether Scaffold deploys this contract on this network.
    pub deploys: bool,
}

/// Resolves networks once each, so a broken parent is reported once rather
/// than once per child.
pub struct Resolver<'c> {
    config: &'c Config,
    cache: HashMap<String, Option<Network>>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'c> Resolver<'c> {
    pub fn new(config: &'c Config) -> Self {
        Self {
            config,
            cache: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Validate every declared network and every contract on every network it
    /// lists. With `selected`, also require each contract to support it.
    pub fn validate_all(&mut self, selected: Option<&str>) {
        let config = self.config;
        for name in config.networks.keys() {
            self.network(name);
        }
        if let Some(sel) = selected
            && !config.networks.contains_key(sel)
        {
            self.unknown_network(sel, None);
        }
        self.check_export_collisions();
        for (key, def) in config.contracts.iter() {
            let name = key.value.as_str();
            let Some(networks) = def.networks.as_ref().filter(|n| !n.value.is_empty()) else {
                self.diagnostics.push(
                    Diagnostic::warning(
                        Code::ContractWithoutNetworks,
                        format!("contract `{name}` lists no networks, so it is never built"),
                    )
                    .at(span(key))
                    .help("add `networks:` with at least one entry, e.g. `local: {}`"),
                );
                continue;
            };
            self.check_client_deploy_keys(name, def);
            for net in networks.value.keys() {
                self.contract(name, net);
            }
        }
    }

    /// Resolve a declared network. `None` if undeclared or broken.
    pub fn network(&mut self, name: &str) -> Option<Network> {
        self.network_in_chain(name, &mut Vec::new())
    }

    fn network_in_chain(&mut self, name: &str, chain: &mut Vec<String>) -> Option<Network> {
        if let Some(cached) = self.cache.get(name) {
            return cached.clone();
        }
        let config = self.config;
        let (key, def) = config.networks.entry(name)?;
        chain.push(name.to_string());
        let base = self.base(name, def, chain);
        chain.pop();
        let resolved = base.map(|base| apply(name, def, base));
        if let Some(net) = &resolved {
            self.check_network(net, def, span(key));
        }
        self.cache.insert(name.to_string(), resolved.clone());
        resolved
    }

    /// The network a definition builds on: its `extends` parent, else
    /// built-in defaults for its name (none for custom names).
    fn base(&mut self, name: &str, def: &NetworkDef, chain: &mut Vec<String>) -> Option<Network> {
        let Some(parent) = &def.extends else {
            return Some(Network::builtin(name));
        };
        if chain.contains(&parent.value) {
            let cycle = [chain.as_slice(), std::slice::from_ref(&parent.value)]
                .concat()
                .join(" → ");
            self.diagnostics.push(
                Diagnostic::error(
                    Code::ExtendsChain,
                    format!("`extends` forms a cycle: {cycle}"),
                )
                .at(span(parent)),
            );
            return None;
        }
        if chain.len() > MAX_EXTENDS_DEPTH {
            self.diagnostics.push(
                Diagnostic::error(
                    Code::ExtendsChain,
                    format!(
                        "`extends` chain is deeper than {MAX_EXTENDS_DEPTH}: {} → {}",
                        chain.join(" → "),
                        parent.value
                    ),
                )
                .at(span(parent)),
            );
            return None;
        }
        if !self.config.networks.contains_key(&parent.value) {
            self.unknown_network(&parent.value, Some(span(parent)));
            return None;
        }
        let mut base = self.network_in_chain(&parent.value, chain)?;
        base.extends.insert(0, parent.value.clone());
        Some(base)
    }

    fn check_network(&mut self, net: &Network, def: &NetworkDef, key: Span) {
        let missing: Vec<&str> = [
            ("rpc-url", net.rpc_url.is_none()),
            ("network-passphrase", net.passphrase.is_none()),
        ]
        .into_iter()
        .filter_map(|(k, m)| m.then_some(k))
        .collect();
        if !missing.is_empty() {
            let help = if DEFAULTS.contains_key(net.name.as_str()) {
                "stellar-cli has no usable default for this network; set it explicitly"
            } else {
                "custom networks must set these, or `extends` a network that does"
            };
            self.diagnostics.push(
                Diagnostic::error(
                    Code::MissingNetworkSettings,
                    format!("network `{}` has no {}", net.name, missing.join(" or ")),
                )
                .at(key)
                .help(help),
            );
        }
        if let Some(d) = &net.explicit_default_account
            && !net.accounts.contains(d)
        {
            let at = def.default_account.as_ref().map_or(key, span);
            let inherited = if def.default_account.is_none() {
                " (inherited via `extends`)"
            } else {
                ""
            };
            self.diagnostics.push(
                Diagnostic::error(
                    Code::DefaultAccount,
                    format!(
                        "`default-account: {d}`{inherited} is not in network `{}`'s accounts",
                        net.name
                    ),
                )
                .at(at)
                .help("list it under `accounts`, or set `default-account` to one that is"),
            );
        }
        let start_at = def.start_container.as_ref().map_or(key, span);
        if net.start_container() && net.is_public() {
            self.diagnostics.push(
                Diagnostic::error(
                    Code::StartContainerOnPublic,
                    format!(
                        "network `{}` uses a public passphrase but sets `start-container: true`",
                        net.name
                    ),
                )
                .at(start_at)
                .help("a local container can only run a standalone network"),
            );
        } else if net.explicit_start_container == Some(true)
            && !net.is_local()
            && net.literal_passphrase().is_some()
        {
            self.diagnostics.push(
                Diagnostic::warning(Code::StartContainerNonLocal, format!("network `{}` starts a container but its passphrase is not the standalone one", net.name))
                    .at(start_at),
            );
        }
        if net.allow_deploy() && net.literal_passphrase() == Some(passphrase::MAINNET) {
            self.diagnostics.push(
                Diagnostic::warning(
                    Code::DeployToMainnet,
                    format!("network `{}` allows deploys to mainnet", net.name),
                )
                .at(def.allow_deploy.as_ref().map_or(key, span))
                .help("make sure this is intended; deploys cost real funds and cannot be undone"),
            );
        }
    }

    fn unknown_network(&mut self, name: &str, at: Option<Span>) {
        let mut d = Diagnostic::error(
            Code::UnknownNetwork,
            format!("network `{name}` is not declared under `networks:`"),
        );
        if let Some(at) = at {
            d = d.at(at);
        }
        d = match suggest(name, self.config.networks.keys()) {
            Some(s) => d.help(format!("did you mean `{s}`?")),
            None => d.help(format!("add `{name}: {{}}` under `networks:`")),
        };
        self.diagnostics.push(d);
    }

    /// Two contract names that map to the same camelCase export would clash
    /// in the generated clients `index.ts`.
    fn check_export_collisions(&mut self) {
        use heck::ToLowerCamelCase;
        let mut seen: HashMap<String, &str> = HashMap::new();
        for (key, _) in self.config.contracts.iter() {
            let export = key.value.to_lower_camel_case();
            if let Some(first) = seen.get(&export) {
                self.diagnostics.push(
                    Diagnostic::error(
                        Code::InvalidName,
                        format!(
                            "contracts `{first}` and `{}` both generate the export `{export}`",
                            key.value
                        ),
                    )
                    .at(span(key))
                    .help("rename one of them"),
                );
            } else {
                seen.insert(export, &key.value);
            }
        }
    }

    /// Deploy keys at client level are defaults for networks that deploy. They
    /// are an error only when the client-level `type` itself can never deploy.
    fn check_client_deploy_keys(&mut self, name: &str, defaults: &ContractSettings) {
        let Some(ty) = &defaults.ty else { return };
        if ty.value == SourceType::Workspace || defaults.from_network.is_some() {
            return;
        }
        for (key, at) in defaults.deploy_keys() {
            self.diagnostics.push(
                Diagnostic::error(
                    Code::DeployKeyNotAllowed,
                    format!("`{key}` has no effect: contract `{name}` has `type: {}`, which Scaffold does not deploy", ty.value.as_str()),
                )
                .at(at)
                .help("deploy settings apply only to `type: workspace` or with `from-network`"),
            );
        }
    }

    /// Resolve and validate one contract on one network. Returns `None` if
    /// the contract or network is broken badly enough that no view exists.
    pub fn contract(&mut self, name: &str, network: &str) -> Option<Contract> {
        let config = self.config;
        let def = config.contracts.get(name)?;
        let (net_key, overrides) = def.networks.as_ref()?.value.entry(network)?;
        let net_span = span(net_key);
        if !config.networks.contains_key(network) {
            self.unknown_network(network, Some(net_span));
            return None;
        }
        let merged = merge(def, overrides);
        let (Some(ty), Some(source)) = (&merged.ty, &merged.source) else {
            self.diagnostics.push(
                Diagnostic::error(
                    Code::MissingSource,
                    format!("contract `{name}` has no `type` and `source` for network `{network}`"),
                )
                .at(net_span)
                .help("set them on the contract, or under this network entry"),
            );
            return None;
        };
        let net = self.network(network)?;
        let deploys = self.check_contract(name, &net, &merged, overrides, ty, net_span);
        let mut contract = Contract {
            name: name.to_string(),
            ty: ty.value,
            source: source.value.clone(),
            from_network: merged.from_network.as_ref().map(|s| s.value.clone()),
            signer: None,
            args: SpannedMap::default(),
            after_deploy: Vec::new(),
            after_deploy_script: None,
            deploys,
        };
        // Client-level deploy settings are inherited only where Scaffold
        // actually deploys; a network that references an existing contract
        // ignores them.
        if deploys {
            contract.signer = merged
                .signer
                .map(|s| s.value)
                .or_else(|| net.default_account().map(str::to_string));
            contract.args = merged.args.map(|a| a.value).unwrap_or_default();
            contract.after_deploy = merged
                .after_deploy
                .map(|m| m.value.into_iter().map(|s| s.value).collect())
                .unwrap_or_default();
            contract.after_deploy_script = merged.after_deploy_script.map(|s| s.value);
        }
        Some(contract)
    }

    /// Rules that need both the merged contract and its resolved network.
    /// Returns whether Scaffold deploys the contract here.
    ///
    /// Only deploy keys written on this network entry are flagged when the
    /// contract isn't deployed here; client-level ones are checked once in
    /// [`Self::check_client_deploy_keys`].
    fn check_contract(
        &mut self,
        name: &str,
        net: &Network,
        s: &ContractSettings,
        overrides: &ContractSettings,
        ty: &Spanned<SourceType>,
        net_span: Span,
    ) -> bool {
        let deployable = ty.value == SourceType::Workspace || s.from_network.is_some();
        if !deployable {
            for (key, at) in overrides.deploy_keys() {
                self.diagnostics.push(
                    Diagnostic::error(Code::DeployKeyNotAllowed, format!("`{key}` has no effect: contract `{name}` is not deployed by Scaffold on `{}`", net.name))
                        .at(at)
                        .help("deploy settings apply only to `type: workspace` or with `from-network`"),
                );
            }
        }
        if let Some(from) = &s.from_network {
            if !ty.value.allows_from_network() {
                self.diagnostics.push(
                    Diagnostic::error(
                        Code::FromNetwork,
                        format!(
                            "`from-network` cannot be used with `type: {}`",
                            ty.value.as_str()
                        ),
                    )
                    .at(span(from))
                    .help("use it with `contract`, `registry`, or `wasm-hash`"),
                );
            } else if from.value == net.name {
                self.diagnostics.push(
                    Diagnostic::error(
                        Code::FromNetwork,
                        "`from-network` names the network it is on",
                    )
                    .at(span(from)),
                );
            } else if !self.config.networks.contains_key(&from.value) {
                self.unknown_network(&from.value, Some(span(from)));
            }
        }
        if deployable && !net.allow_deploy() {
            self.diagnostics.push(
                Diagnostic::error(Code::DeployNotAllowed, format!("contract `{name}` would be deployed to `{}`, which has deploys disabled", net.name))
                    .at(net_span)
                    .help("reference an existing deployment (`type: contract` or `registry`), use `type: wasm-file` for a types-only client, or set `allow-deploy: true` on the network"),
            );
            return false;
        }
        if deployable {
            self.check_accounts(name, net, s, net_span);
        }
        deployable
    }

    fn check_accounts(&mut self, name: &str, net: &Network, s: &ContractSettings, net_span: Span) {
        let mut refs: Vec<(String, Span)> = Vec::new();
        if let Some(signer) = &s.signer {
            refs.push((signer.value.clone(), span(signer)));
        } else if net.default_account().is_none() {
            self.diagnostics.push(
                Diagnostic::error(Code::UnknownAccount, format!("contract `{name}` is deployed to `{}`, which lists no accounts to sign with", net.name))
                    .at(net_span)
                    .help(format!("add `accounts:` to network `{}`, or set `signer`", net.name)),
            );
        }
        if let Some(args) = &s.args {
            for (_, v) in args.value.iter() {
                collect_account_refs(v, &mut refs);
            }
        }
        for (account, at) in refs {
            if !net.accounts.contains(&account) {
                self.diagnostics.push(
                    Diagnostic::error(
                        Code::UnknownAccount,
                        format!(
                            "account `{account}` is not listed in network `{}`'s accounts",
                            net.name
                        ),
                    )
                    .at(at)
                    .help(format!("add it to `networks.{}.accounts`", net.name)),
                );
            }
        }
    }
}

/// Apply a definition's explicit keys over its resolved base.
fn apply(name: &str, def: &NetworkDef, mut net: Network) -> Network {
    net.name = name.to_string();
    if let Some(t) = &def.rpc_url {
        net.rpc_url = Some(Template::new(&t.value));
    }
    if let Some(t) = &def.network_passphrase {
        net.passphrase = Some(Template::new(&t.value));
    }
    for (k, v) in def.rpc_headers.iter() {
        net.rpc_headers.retain(|(existing, _)| *existing != k.value);
        net.rpc_headers
            .push((k.value.clone(), Template::new(&v.value)));
    }
    if let Some(accounts) = &def.accounts {
        net.accounts = accounts.iter().map(|a| a.value.clone()).collect();
    }
    if let Some(d) = &def.default_account {
        net.explicit_default_account = Some(d.value.clone());
    }
    if let Some(b) = &def.start_container {
        net.explicit_start_container = Some(b.value);
    }
    if let Some(b) = &def.allow_deploy {
        net.explicit_allow_deploy = Some(b.value);
    }
    net
}

fn collect_account_refs(v: &Spanned<Value>, out: &mut Vec<(String, Span)>) {
    match &v.value {
        Value::Str(s) => {
            if let Ok(segments) = interpolate::parse(s) {
                out.extend(interpolate::accounts(&segments).map(|a| (a.to_string(), span(v))));
            }
        }
        Value::Seq(items) => items.iter().for_each(|i| collect_account_refs(i, out)),
        Value::Map(map) => map.iter().for_each(|(_, i)| collect_account_refs(i, out)),
        _ => {}
    }
}

/// Per-network settings over client-level defaults. `type` and `source`
/// always travel together, which `lint` enforces.
fn merge(defaults: &ContractSettings, over: &ContractSettings) -> ContractSettings {
    let args = match (&defaults.args, &over.args) {
        (Some(d), Some(o)) => {
            let mut map = d.value.clone();
            for (k, v) in o.value.iter() {
                map.0.retain(|(existing, _)| existing.value != k.value);
                map.0.push((k.clone(), v.clone()));
            }
            Some(Spanned::new(map, o.referenced, o.defined))
        }
        (d, o) => o.clone().or_else(|| d.clone()),
    };
    ContractSettings {
        ty: over.ty.clone().or_else(|| defaults.ty.clone()),
        source: over.source.clone().or_else(|| defaults.source.clone()),
        from_network: over
            .from_network
            .clone()
            .or_else(|| defaults.from_network.clone()),
        signer: over.signer.clone().or_else(|| defaults.signer.clone()),
        args,
        after_deploy: over
            .after_deploy
            .clone()
            .or_else(|| defaults.after_deploy.clone()),
        after_deploy_script: over
            .after_deploy_script
            .clone()
            .or_else(|| defaults.after_deploy_script.clone()),
        networks: None,
    }
}
