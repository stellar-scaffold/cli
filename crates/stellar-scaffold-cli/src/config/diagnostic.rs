//! Structured diagnostics for `scaffold.yml`.
//!
//! Every problem found while loading or validating the config is a
//! [`Diagnostic`] with a stable code, a severity, an optional source span, and
//! an optional one-line remedy. The same values back `scaffold config check`,
//! `scaffold config show`, and `doctor`, so wording stays identical everywhere.

use std::fmt::Write as _;

use serde_saphyr::{Location, Spanned};

/// A 1-based source position and its length in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct Span {
    pub line: u64,
    pub col: u64,
    pub len: u64,
}

impl From<Location> for Span {
    fn from(loc: Location) -> Self {
        Self {
            line: loc.line(),
            col: loc.column(),
            len: loc.span().len(),
        }
    }
}

/// Where a parsed value was written. Aliases report the alias, not the anchor.
pub fn span<T>(s: &Spanned<T>) -> Span {
    s.referenced.into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// Stable diagnostic codes. Never rename a code's slug: slugs are referenced
/// from docs, reported by `doctor`, and may be matched by CI tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// YAML syntax errors and values of the wrong type.
    Parse,
    /// A mapping defines the same key twice.
    DuplicateKey,
    /// `version` is missing or not supported by this CLI.
    SchemaVersion,
    /// A key is not part of the schema.
    UnknownKey,
    /// A referenced network is not declared.
    UnknownNetwork,
    /// A custom network lacks `rpc-url` or `network-passphrase`.
    MissingNetworkSettings,
    /// `extends` forms a cycle or chain deeper than allowed.
    ExtendsChain,
    /// `start-container: true` on a public network.
    StartContainerOnPublic,
    /// `default-account` is not listed in `accounts`.
    DefaultAccount,
    /// A contract has no `type`/`source`, or sets only one of the pair.
    MissingSource,
    /// A `type` or `source` value is not valid.
    InvalidSource,
    /// A `workspace` source names a crate that is not in `contracts-dir`.
    CrateNotFound,
    /// A deploy-only key is set on a contract that cannot be deployed.
    DeployKeyNotAllowed,
    /// `from-network` is misused.
    FromNetwork,
    /// A `workspace` contract targets a network where deploys are disabled.
    DeployNotAllowed,
    /// `after-deploy-script` is missing or not executable.
    AfterDeployScript,
    /// A `${…}` expression is malformed or cannot be resolved.
    Interpolation,
    /// A signer or `${account.…}` names an account the network doesn't list.
    UnknownAccount,
    /// A network, contract, account, or method name is not valid.
    InvalidName,
    /// Deploys are enabled on a mainnet-passphrase network.
    DeployToMainnet,
    /// A third-party source has no pinned version.
    UnpinnedVersion,
    /// `start-container: true` on a non-local passphrase.
    StartContainerNonLocal,
    /// A contract has no network entries, so it is never built.
    ContractWithoutNetworks,
    /// A `wasm-file` source does not exist yet.
    WasmFileMissing,
    /// The cargo workspace could not be read, so crate checks were skipped.
    WorkspaceUnavailable,
}

impl Code {
    /// The stable kebab-case identifier, matching `doctor`'s check names.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Parse => "parse-error",
            Self::DuplicateKey => "duplicate-key",
            Self::SchemaVersion => "schema-version",
            Self::UnknownKey => "unknown-key",
            Self::UnknownNetwork => "unknown-network",
            Self::MissingNetworkSettings => "missing-network-settings",
            Self::ExtendsChain => "extends-chain",
            Self::StartContainerOnPublic => "start-container-public",
            Self::DefaultAccount => "default-account",
            Self::MissingSource => "missing-source",
            Self::InvalidSource => "invalid-source",
            Self::CrateNotFound => "crate-not-found",
            Self::DeployKeyNotAllowed => "deploy-key-not-allowed",
            Self::FromNetwork => "from-network",
            Self::DeployNotAllowed => "deploy-not-allowed",
            Self::AfterDeployScript => "after-deploy-script",
            Self::Interpolation => "interpolation",
            Self::UnknownAccount => "unknown-account",
            Self::InvalidName => "invalid-name",
            Self::DeployToMainnet => "deploy-to-mainnet",
            Self::UnpinnedVersion => "unpinned-version",
            Self::StartContainerNonLocal => "start-container-non-local",
            Self::ContractWithoutNetworks => "contract-without-networks",
            Self::WasmFileMissing => "wasm-file-missing",
            Self::WorkspaceUnavailable => "workspace-unavailable",
        }
    }
}

impl serde::Serialize for Code {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.slug())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnostic {
    pub code: Code,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            span: None,
            help: None,
        }
    }

    pub fn warning(code: Code, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(code, message)
        }
    }

    /// Attach a source location.
    #[must_use]
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Attach a one-line remedy.
    #[must_use]
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// `file:line:col`, or just `file` without a span.
    pub fn location(&self, file: &str) -> String {
        match self.span {
            Some(s) => format!("{file}:{}:{}", s.line, s.col),
            None => file.to_string(),
        }
    }

    /// Render rustc-style, with the offending line and a caret underline:
    ///
    /// ```text
    /// error[unknown-key]: unknown key `rpc_url` in network `testnet`
    ///   --> scaffold.yml:12:5
    ///    |
    /// 12 |     rpc_url: https://example.com
    ///    |     ^^^^^^^
    ///    = help: did you mean `rpc-url`?
    /// ```
    pub fn render(&self, file: &str, source: &str) -> String {
        let label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let mut out = format!("{label}[{}]: {}\n", self.code.slug(), self.message);
        let gutter = self.span.map_or(1, |s| s.line.to_string().len());
        let pad = " ".repeat(gutter);
        if let Some(span) = self.span {
            let _ = writeln!(out, "{pad}--> {}", self.location(file));
            let line = usize::try_from(span.line).unwrap_or(usize::MAX);
            if let Some(text) = source.lines().nth(line.saturating_sub(1)) {
                let col = usize::try_from(span.col).unwrap_or(1).max(1);
                // Multi-line values (block mappings) underline to end of line.
                let rest = text.chars().count().saturating_sub(col - 1);
                let width = usize::try_from(span.len).unwrap_or(1).clamp(1, rest.max(1));
                let _ = writeln!(out, "{pad} |");
                let _ = writeln!(out, "{} | {text}", span.line);
                let _ = writeln!(out, "{pad} | {}{}", " ".repeat(col - 1), "^".repeat(width));
            }
        }
        if let Some(help) = &self.help {
            let _ = writeln!(out, "{pad} = help: {help}");
        }
        out
    }
}

/// Keys from older config formats, mapped to where they moved.
const MOVED_KEYS: &[(&str, &str)] = &[
    (
        "config",
        "its settings moved to `project:` (e.g. `contracts_dir` → `contracts-dir`)",
    ),
    ("contract-clients", "use `contracts:`"),
    ("run-locally", "use `start-container`"),
    ("run_locally", "use `start-container`"),
    (
        "constructor_args",
        "use `args:`, a map of constructor parameter names to values",
    ),
    ("id", "set `type: contract` and `source: C…`"),
    ("client", "remove it; every listed contract gets a client"),
    (
        "optimize",
        "builds are optimized by default; pass `--optimize=false` to `stellar scaffold build` to opt out",
    ),
];

/// Convert a serde-saphyr error into a diagnostic, adding "did you mean"
/// hints to unknown keys and unknown `type` values.
pub fn from_parse_error(e: &serde_saphyr::Error) -> Diagnostic {
    let text = e.to_string();
    // Messages end with " at line N, column M"; the span carries that instead.
    let message = text
        .rsplit_once(" at line ")
        .map_or(text.as_str(), |(m, _)| m)
        .trim_end_matches(", set DuplicateKeyPolicy in Options if acceptable");
    let mut d = if message.starts_with("duplicate mapping key") {
        Diagnostic::error(Code::DuplicateKey, message)
    } else if let Some((name, expected)) = unknown_name(message, "unknown field") {
        let mut d = Diagnostic::error(Code::UnknownKey, format!("unknown key `{name}`"));
        if let Some((_, moved)) = MOVED_KEYS.iter().find(|(k, _)| *k == name) {
            d = d.help(format!("`{name}` is no longer supported; {moved}"));
        } else if let Some(s) = suggest(name, expected.iter().copied()) {
            d = d.help(format!("did you mean `{s}`?"));
        } else {
            d = d.help(format!("expected one of: {}", expected.join(", ")));
        }
        d
    } else if let Some((name, expected)) = unknown_name(message, "unknown variant") {
        let mut d = Diagnostic::error(
            Code::InvalidSource,
            format!("unknown contract type `{name}`"),
        );
        d = match suggest(name, expected.iter().copied()) {
            Some(s) => d.help(format!("did you mean `{s}`?")),
            None => d.help(format!("expected one of: {}", expected.join(", "))),
        };
        d
    } else {
        Diagnostic::error(Code::Parse, message)
    };
    if let Some(loc) = e.location() {
        d = d.at(loc.into());
    }
    d
}

/// Parse serde's "unknown field `x`, expected one of `a`, `b`" (and the
/// `unknown variant` form) into the name and the expected names.
fn unknown_name<'m>(message: &'m str, prefix: &str) -> Option<(&'m str, Vec<&'m str>)> {
    let rest = message.strip_prefix(prefix)?.trim_start();
    let rest = rest.strip_prefix('`')?;
    let (name, rest) = rest.split_once('`')?;
    let expected = rest
        .split_once("expected")
        .map(|(_, list)| {
            list.trim_start_matches(" one of")
                .split(',')
                .map(|s| s.trim().trim_matches('`'))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    Some((name, expected))
}

/// Suggest the closest candidate to `input`, if any is near enough to be a
/// plausible typo.
pub fn suggest<'a>(input: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let normalized = input.replace('_', "-").to_lowercase();
    candidates
        .into_iter()
        .map(|c| (c, strsim::levenshtein(&normalized, c)))
        .filter(|(c, d)| *d <= (c.len() / 3).max(1))
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_points_at_span() {
        let d = Diagnostic::error(Code::UnknownKey, "unknown key `rpc_url`")
            .at(Span {
                line: 2,
                col: 3,
                len: 7,
            })
            .help("did you mean `rpc-url`?");
        let out = d.render("scaffold.yml", "a:\n  rpc_url: x\n");
        assert_eq!(
            out,
            "error[unknown-key]: unknown key `rpc_url`\n \
             --> scaffold.yml:2:3\n  |\n\
             2 |   rpc_url: x\n  |   ^^^^^^^\n  = help: did you mean `rpc-url`?\n"
        );
    }

    #[test]
    fn suggest_handles_snake_case() {
        assert_eq!(suggest("rpc_url", ["rpc-url", "accounts"]), Some("rpc-url"));
        assert_eq!(suggest("banana", ["rpc-url", "accounts"]), None);
    }

    #[test]
    fn unknown_name_parses_serde_messages() {
        let (name, expected) = unknown_name(
            "unknown field `rpc_url`, expected one of rpc-url, accounts",
            "unknown field",
        )
        .unwrap();
        assert_eq!(name, "rpc_url");
        assert_eq!(expected, vec!["rpc-url", "accounts"]);
        let (name, expected) = unknown_name(
            "unknown variant `contrct`, expected one of `workspace`, `contract`",
            "unknown variant",
        )
        .unwrap();
        assert_eq!(name, "contrct");
        assert_eq!(expected, vec!["workspace", "contract"]);
    }
}
