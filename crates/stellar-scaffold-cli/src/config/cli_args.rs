//! Constructor arguments as stellar-cli command-line arguments.
//!
//! stellar-cli converts each `--name value` pair against the contract's
//! `__constructor` spec, so values only need to be rendered as the strings a
//! user would type. The result is an argument list handed straight to
//! stellar-cli's parser; no shell ever sees it.

use serde_saphyr::Spanned;

use super::interpolate::{self, Context};
use super::schema::{SpannedMap, Value};

/// `--name value` pairs for `args`, in the order written. `null` values are
/// omitted, which stellar-cli reads as `None` for `Option` parameters.
pub fn constructor_args(
    args: &SpannedMap<Spanned<Value>>,
    ctx: &Context,
) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (key, value) in args.iter() {
        if value.value == Value::Null {
            continue;
        }
        out.push(format!("--{}", key.value));
        out.push(render(&value.value, ctx)?);
    }
    Ok(out)
}

/// A value as stellar-cli expects it: scalars as plain text, lists and maps
/// (vectors, structs, maps) as JSON.
fn render(value: &Value, ctx: &Context) -> Result<String, String> {
    Ok(match value {
        Value::Str(s) => interpolate_str(s, ctx)?,
        Value::Int(n) => n.clone(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Seq(_) | Value::Map(_) => value
            .to_json_with(&mut |s| interpolate_str(s, ctx))?
            .to_string(),
    })
}

fn interpolate_str(s: &str, ctx: &Context) -> Result<String, String> {
    interpolate::resolve(&interpolate::parse(s)?, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::interpolate::Mode;
    use crate::config::schema;

    fn args_of(yaml: &str) -> SpannedMap<Spanned<Value>> {
        let config =
            schema::parse(&format!("version: 2\ncontracts:\n  c:\n    args: {yaml}\n")).unwrap();
        config
            .contracts
            .get("c")
            .unwrap()
            .args
            .clone()
            .unwrap()
            .value
    }

    fn render_args(yaml: &str) -> Vec<String> {
        let env = |name: &str| (name == "SYMBOL").then(|| "EXT".to_string());
        let ctx = Context {
            env: &env,
            network: Some(("local", "http://rpc", "pass")),
            mode: Mode::Final,
        };
        constructor_args(&args_of(yaml), &ctx).unwrap()
    }

    #[test]
    fn scalars_render_as_text() {
        assert_eq!(
            render_args(
                "{ name: Token, supply: 1000000000000000000000000, live: true, rate: 1.5 }"
            ),
            vec![
                "--name",
                "Token",
                "--supply",
                "1000000000000000000000000",
                "--live",
                "true",
                "--rate",
                "1.5"
            ]
        );
    }

    #[test]
    fn references_resolve_to_aliases_and_values() {
        assert_eq!(
            render_args(
                r#"{ admin: "${account.me}", symbol: "${env.SYMBOL}", net: "${network.name}" }"#
            ),
            vec!["--admin", "me", "--symbol", "EXT", "--net", "local"]
        );
    }

    #[test]
    fn nulls_are_omitted() {
        assert_eq!(render_args("{ a: 1, b: ~ }"), vec!["--a", "1"]);
    }

    #[test]
    fn lists_and_maps_render_as_json() {
        assert_eq!(
            render_args(
                r#"{ v: [1, 170141183460469231731687303715884105727], s: { owner: "${account.me}", n: 2 } }"#
            ),
            vec![
                "--v",
                r#"[1,"170141183460469231731687303715884105727"]"#,
                "--s",
                r#"{"owner":"me","n":2}"#,
            ]
        );
    }

    #[test]
    fn shell_syntax_is_passed_through_untouched() {
        assert_eq!(
            render_args(r#"{ name: "$(rm -rf ~)" }"#),
            vec!["--name", "$(rm -rf ~)"]
        );
    }
}
