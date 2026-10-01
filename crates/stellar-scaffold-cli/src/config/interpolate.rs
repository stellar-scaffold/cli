//! `${namespace.path}` interpolation in string values.
//!
//! The full grammar is parsed up front, including namespaces that are reserved
//! but not yet resolvable, so adding them later is purely additive. `$${`
//! escapes a literal `${`.

/// Namespaces this CLI can resolve.
const IMPLEMENTED: &[&str] = &["account", "env", "network"];
/// Namespaces reserved for future use: parsed, but rejected.
const RESERVED: &[&str] = &["contract", "wasm-hash", "registry"];
/// Keys available under `${network.…}`.
const NETWORK_KEYS: &[&str] = &["name", "rpc-url", "passphrase"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    Account,
    Env,
    Network,
}

impl Namespace {
    /// The namespace's spelling in the template
    pub fn as_str(self) -> &'static str {
        match self {
            Namespace::Account => "account",
            Namespace::Env => "env",
            Namespace::Network => "network",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Lit(String),
    Ref {
        ns: Namespace,
        path: String,
        /// `${env.VAR:-default}` fallback. Only valid for `env`.
        default: Option<String>,
    },
}

/// Parse a string into literal and reference segments.
pub fn parse(input: &str) -> Result<Vec<Segment>, String> {
    let mut segments = Vec::new();
    let mut lit = String::new();
    let mut rest = input;
    while let Some(i) = rest.find('$') {
        lit.push_str(&rest[..i]);
        let after = &rest[i..];
        if let Some(tail) = after.strip_prefix("$${") {
            lit.push_str("${");
            rest = tail;
        } else if let Some(tail) = after.strip_prefix("${") {
            let end = tail
                .find('}')
                .ok_or_else(|| format!("unterminated `${{` in `{input}`"))?;
            if !lit.is_empty() {
                segments.push(Segment::Lit(std::mem::take(&mut lit)));
            }
            segments.push(parse_ref(&tail[..end])?);
            rest = &tail[end + 1..];
        } else {
            lit.push('$');
            rest = &after[1..];
        }
    }
    lit.push_str(rest);
    if !lit.is_empty() {
        segments.push(Segment::Lit(lit));
    }
    Ok(segments)
}

fn parse_ref(inner: &str) -> Result<Segment, String> {
    let (expr, default) = match inner.split_once(":-") {
        Some((expr, default)) => (expr, Some(default.to_string())),
        None => (inner, None),
    };
    let Some((ns, path)) = expr.split_once('.') else {
        return Err(format!(
            "`${{{inner}}}` must have the form `${{namespace.name}}`"
        ));
    };
    let ns = match ns {
        "account" => Namespace::Account,
        "env" => Namespace::Env,
        "network" => Namespace::Network,
        _ if RESERVED.contains(&ns) => {
            return Err(format!(
                "`${{{ns}.…}}` is reserved for a future version and not supported yet"
            ));
        }
        _ => {
            let hint = super::diagnostic::suggest(ns, IMPLEMENTED.iter().copied())
                .map(|s| format!("did you mean `{s}`? "))
                .unwrap_or_default();
            return Err(format!(
                "unknown namespace `{ns}`; {hint}supported: {}",
                IMPLEMENTED.join(", ")
            ));
        }
    };
    let valid_path = match ns {
        Namespace::Env => {
            path.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && path.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        Namespace::Account => super::is_valid_name(path),
        Namespace::Network => NETWORK_KEYS.contains(&path),
    };
    if !valid_path {
        let expected = match ns {
            Namespace::Env => "an environment variable name".to_string(),
            Namespace::Account => "an account name".to_string(),
            Namespace::Network => format!("one of {}", NETWORK_KEYS.join(", ")),
        };
        return Err(format!("`{path}` in `${{{inner}}}` is not {expected}"));
    }
    if default.is_some() && ns != Namespace::Env {
        return Err(format!(
            "`:-` defaults are only allowed for `${{env.…}}`, not in `${{{inner}}}`"
        ));
    }
    Ok(Segment::Ref {
        ns,
        path: path.to_string(),
        default,
    })
}

/// Values available while resolving a string.
pub struct Context<'a> {
    pub env: &'a dyn Fn(&str) -> Option<String>,
    /// `(name, rpc-url, passphrase)` of the network being resolved, when the
    /// string may reference it.
    pub network: Option<(&'a str, &'a str, &'a str)>,
    pub mode: Mode,
}

/// What a resolved string is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// For showing config back to the user: `$${` escapes and
    /// `${account.…}` references stay as written, so output is valid config.
    Display,
    /// For passing to stellar-cli: escapes become literal `${` and
    /// `${account.x}` becomes the alias `x`, which stellar-cli resolves to
    /// the account's address.
    Final,
}

/// Resolve `env` and `network` references. `account` references never
/// resolve to an address here: addresses live in the keystore and may not
/// exist until build generates them. See [`Mode`].
pub fn resolve(segments: &[Segment], ctx: &Context) -> Result<String, String> {
    let mut out = String::new();
    for segment in segments {
        match segment {
            Segment::Lit(s) if ctx.mode == Mode::Display => out.push_str(&s.replace("${", "$${")),
            Segment::Lit(s) => out.push_str(s),
            Segment::Ref {
                ns: Namespace::Env,
                path,
                default,
            } => match ((ctx.env)(path), default) {
                (Some(v), _) => out.push_str(&v),
                (None, Some(d)) => out.push_str(d),
                (None, None) => {
                    return Err(format!("environment variable `{path}` is not set"));
                }
            },
            Segment::Ref {
                ns: Namespace::Network,
                path,
                ..
            } => {
                let Some((name, rpc, passphrase)) = ctx.network else {
                    return Err("`${network.…}` is not available here".to_string());
                };
                out.push_str(match path.as_str() {
                    "name" => name,
                    "rpc-url" => rpc,
                    _ => passphrase,
                });
            }
            Segment::Ref {
                ns: Namespace::Account,
                path,
                ..
            } => {
                if ctx.mode == Mode::Display {
                    out.push_str("${account.");
                    out.push_str(path);
                    out.push('}');
                } else {
                    out.push_str(path);
                }
            }
        }
    }
    Ok(out)
}

/// Account names referenced by `${account.…}` in `segments`.
pub fn accounts(segments: &[Segment]) -> impl Iterator<Item = &str> {
    segments.iter().filter_map(|s| match s {
        Segment::Ref {
            ns: Namespace::Account,
            path,
            ..
        } => Some(path.as_str()),
        _ => None,
    })
}

/// Namespaces referenced in `segments`.
pub fn namespaces(segments: &[Segment]) -> impl Iterator<Item = Namespace> + '_ {
    segments.iter().filter_map(|s| match s {
        Segment::Ref { ns, .. } => Some(*ns),
        Segment::Lit(_) => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Option<String> {
        (name == "TOKEN").then(|| "secret".to_string())
    }

    fn resolve_str(s: &str) -> Result<String, String> {
        let ctx = Context {
            env: &env,
            network: Some((
                "preview",
                "https://rpc",
                "Test SDF Network ; September 2015",
            )),
            mode: Mode::Display,
        };
        resolve(&parse(s)?, &ctx)
    }

    #[test]
    fn literal_only() {
        assert_eq!(parse("plain").unwrap(), vec![Segment::Lit("plain".into())]);
        assert_eq!(resolve_str("cost: $5").unwrap(), "cost: $5");
    }

    #[test]
    fn env_and_defaults() {
        assert_eq!(resolve_str("Bearer ${env.TOKEN}").unwrap(), "Bearer secret");
        assert_eq!(resolve_str("${env.MISSING:-fallback}").unwrap(), "fallback");
        assert!(resolve_str("${env.MISSING}").is_err());
    }

    #[test]
    fn network_values() {
        assert_eq!(
            resolve_str("${network.name}@${network.rpc-url}").unwrap(),
            "preview@https://rpc"
        );
        assert!(parse("${network.url}").is_err());
    }

    #[test]
    fn accounts_stay_symbolic() {
        let segs = parse("${account.admin}").unwrap();
        assert_eq!(accounts(&segs).collect::<Vec<_>>(), vec!["admin"]);
        assert_eq!(resolve_str("${account.admin}").unwrap(), "${account.admin}");
    }

    #[test]
    fn final_mode_unescapes_and_uses_aliases() {
        let ctx = Context {
            env: &env,
            network: None,
            mode: Mode::Final,
        };
        let segs = parse("$${x} ${account.admin}").unwrap();
        assert_eq!(resolve(&segs, &ctx).unwrap(), "${x} admin");
    }

    #[test]
    fn escape_is_literal() {
        assert_eq!(
            parse("$${env.X}").unwrap(),
            vec![Segment::Lit("${env.X}".into())]
        );
    }

    #[test]
    fn grammar_errors() {
        assert!(parse("${env.X").unwrap_err().contains("unterminated"));
        assert!(parse("${nope}").unwrap_err().contains("namespace.name"));
        assert!(parse("${contract.dex}").unwrap_err().contains("reserved"));
        assert!(
            parse("${acount.me}")
                .unwrap_err()
                .contains("did you mean `account`")
        );
        assert!(
            parse("${account.me:-x}")
                .unwrap_err()
                .contains("only allowed")
        );
    }
}
