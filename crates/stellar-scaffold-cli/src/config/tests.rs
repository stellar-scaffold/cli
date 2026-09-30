//! End-to-end tests through [`super::load`] and [`super::resolved_view`].

use std::path::PathBuf;

use super::{Code, Loaded, load, resolved_view};

const CONTRACT: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

fn load_str(yaml: &str, selected: Option<&str>) -> (tempfile::TempDir, Loaded) {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join(super::CONFIG_FILE), yaml).unwrap();
    let crates = vec![(
        "fungible_token".to_string(),
        dir.path().join("contracts/fungible_token"),
    )];
    let loaded = load(dir.path(), Some(&crates), selected);
    (dir, loaded)
}

fn codes(loaded: &Loaded) -> Vec<Code> {
    loaded.diagnostics.iter().map(|d| d.code).collect()
}

fn show(yaml: &str, network: &str, env: &dyn Fn(&str) -> Option<String>) -> serde_yaml::Value {
    let (_dir, loaded) = load_str(yaml, None);
    assert!(!loaded.has_errors(), "{}", loaded.render());
    resolved_view(loaded.config.as_ref().unwrap(), network, env).unwrap()
}

fn no_env(_: &str) -> Option<String> {
    None
}

#[test]
fn full_example_is_clean() {
    let yaml = format!(
        r#"version: 2
project:
  optimize: true
extensions:
  reporter: {{ warn-size-kb: 128 }}
networks:
  local:
    accounts: [me, admin]
    default-account: admin
  testnet:
    accounts: [deployer]
  preview:
    extends: testnet
    accounts: [ci]
    allow-deploy: true
contracts:
  token:
    type: workspace
    source: fungible_token
    args: {{ owner: "${{account.admin}}" }}
    networks:
      local: {{}}
      preview:
        args: {{ owner: "${{account.ci}}" }}
      testnet:
        type: contract
        source: {CONTRACT}
"#
    );
    let (_dir, loaded) = load_str(&yaml, Some("local"));
    assert!(loaded.diagnostics.is_empty(), "{}", loaded.render());
}

#[test]
fn version_1_is_reported_once() {
    let (_dir, loaded) = load_str("version: 1\nconfig:\n  contracts_dir: contracts\n", None);
    assert_eq!(codes(&loaded), vec![Code::SchemaVersion]);
    assert!(loaded.config.is_none());
}

#[test]
fn unknown_keys_suggest_the_kebab_case_name() {
    let (_dir, loaded) = load_str(
        "version: 2\nnetworks:\n  local:\n    rpc_url: http://x\n",
        None,
    );
    let d = &loaded.diagnostics[0];
    assert_eq!(d.code, Code::UnknownKey);
    assert_eq!(d.help.as_deref(), Some("did you mean `rpc-url`?"));
}

#[test]
fn extends_inherits_builtin_defaults_and_policy() {
    let yaml = "version: 2\nnetworks:\n  testnet: {}\n  preview:\n    extends: testnet\n";
    let view = show(yaml, "preview", &no_env);
    let net = &view["network"];
    assert_eq!(net["rpc-url"], "https://soroban-testnet.stellar.org");
    assert_eq!(net["allow-deploy"], false);
    assert_eq!(net["extends"][0], "testnet");
}

#[test]
fn derived_defaults_follow_the_childs_passphrase() {
    // Inheriting `local`'s derived start-container/allow-deploy would be
    // wrong once the child points at mainnet.
    let yaml = "version: 2\nnetworks:\n  local: {}\n  fork:\n    extends: local\n    rpc-url: https://rpc.example\n    network-passphrase: Public Global Stellar Network ; September 2015\n";
    let view = show(yaml, "fork", &no_env);
    assert_eq!(view["network"]["start-container"], false);
    assert_eq!(view["network"]["allow-deploy"], false);
}

#[test]
fn replacing_accounts_orphans_inherited_default_account() {
    let yaml = "version: 2\nnetworks:\n  local:\n    accounts: [me, admin]\n    default-account: admin\n  other:\n    extends: local\n    accounts: [ci, bot]\n";
    let (_dir, loaded) = load_str(yaml, None);
    assert_eq!(codes(&loaded), vec![Code::DefaultAccount]);
    assert!(loaded.diagnostics[0].message.contains("inherited"));
}

#[test]
fn default_account_falls_back_to_first() {
    let yaml = "version: 2\nnetworks:\n  local:\n    accounts: [me, admin]\n";
    let view = show(yaml, "local", &no_env);
    assert_eq!(view["network"]["default-account"], "me");
}

#[test]
fn extends_cycles_and_depth() {
    let cycle = "version: 2\nnetworks:\n  a: { extends: b }\n  b: { extends: a }\n";
    let (_dir, loaded) = load_str(cycle, None);
    assert_eq!(codes(&loaded), vec![Code::ExtendsChain]);

    let deep = "version: 2\nnetworks:\n  a: { extends: b }\n  b: { extends: c }\n  c: { extends: d }\n  d: { extends: local }\n  local: {}\n";
    let (_dir, loaded) = load_str(deep, None);
    assert_eq!(codes(&loaded), vec![Code::ExtendsChain]);

    let ok = "version: 2\nnetworks:\n  a: { extends: b }\n  b: { extends: c }\n  c: { extends: local }\n  local: {}\n";
    let (_dir, loaded) = load_str(ok, None);
    assert!(loaded.diagnostics.is_empty(), "{}", loaded.render());
}

#[test]
fn extends_requires_a_declared_network() {
    let yaml = "version: 2\nnetworks:\n  preview: { extends: testnet }\n";
    let (_dir, loaded) = load_str(yaml, None);
    assert_eq!(codes(&loaded), vec![Code::UnknownNetwork]);
    assert_eq!(
        loaded.diagnostics[0].help.as_deref(),
        Some("add `testnet: {}` under `networks:`")
    );
}

#[test]
fn inherited_deploy_keys_are_ignored_where_nothing_deploys() {
    let yaml = format!(
        "version: 2\nnetworks:\n  local: {{ accounts: [me] }}\n  mainnet: {{ rpc-url: https://x }}\ncontracts:\n  c:\n    type: workspace\n    source: fungible_token\n    after-deploy: [init]\n    networks:\n      local: {{}}\n      mainnet: {{ type: contract, source: {CONTRACT} }}\n"
    );
    let view = show(&yaml, "mainnet", &no_env);
    let c = &view["contracts"]["c"];
    assert_eq!(c["deploy"], false);
    assert!(c.get("after-deploy").is_none());
}

#[test]
fn deploy_keys_on_a_non_deploying_entry_are_errors() {
    let yaml = format!(
        "version: 2\nnetworks:\n  mainnet: {{ rpc-url: https://x }}\ncontracts:\n  c:\n    networks:\n      mainnet: {{ type: contract, source: {CONTRACT}, after-deploy: [init] }}\n"
    );
    let (_dir, loaded) = load_str(&yaml, None);
    assert_eq!(codes(&loaded), vec![Code::DeployKeyNotAllowed]);
}

#[test]
fn workspace_contract_on_public_network_is_an_error() {
    let yaml = "version: 2\nnetworks:\n  testnet: { accounts: [me] }\ncontracts:\n  c:\n    type: workspace\n    source: fungible_token\n    networks: { testnet: }\n";
    let (_dir, loaded) = load_str(yaml, None);
    assert_eq!(codes(&loaded), vec![Code::DeployNotAllowed]);
}

#[test]
fn selected_network_must_be_covered_by_every_contract() {
    let yaml = "version: 2\nnetworks:\n  local: { accounts: [me] }\n  testnet: {}\ncontracts:\n  c:\n    type: workspace\n    source: fungible_token\n    networks: { local: }\n";
    let (_dir, loaded) = load_str(yaml, Some("testnet"));
    assert_eq!(codes(&loaded), vec![Code::MissingContractNetwork]);
    let (_dir, loaded) = load_str(yaml, None);
    assert!(loaded.diagnostics.is_empty());
}

#[test]
fn args_merge_and_interpolate() {
    let yaml = "version: 2\nnetworks:\n  local: { accounts: [me] }\ncontracts:\n  c:\n    type: workspace\n    source: fungible_token\n    args: { a: 1, b: \"${env.B:-dflt}\" }\n    networks:\n      local:\n        args: { c: \"${network.name}\", n: 170141183460469231731687303715884105727 }\n";
    let view = show(yaml, "local", &no_env);
    let args = &view["contracts"]["c"]["args"];
    assert_eq!(args["a"], 1);
    assert_eq!(args["b"], "dflt");
    assert_eq!(args["c"], "local");
    assert_eq!(args["n"], "170141183460469231731687303715884105727");
}

#[test]
fn missing_env_fails_show_but_not_check() {
    let yaml =
        "version: 2\nnetworks:\n  local:\n    rpc-headers: { Authorization: \"${env.TOKEN}\" }\n";
    let (_dir, loaded) = load_str(yaml, None);
    assert!(loaded.diagnostics.is_empty());
    let err = resolved_view(loaded.config.as_ref().unwrap(), "local", &no_env).unwrap_err();
    assert_eq!(err[0].code, Code::Interpolation);
    let token = |n: &str| (n == "TOKEN").then(|| "t".to_string());
    let view = resolved_view(loaded.config.as_ref().unwrap(), "local", &token).unwrap();
    assert_eq!(view["network"]["rpc-headers"]["Authorization"], "t");
}

#[test]
fn crate_check_is_scoped_to_contracts_dir() {
    let yaml = "version: 2\nproject: { contracts-dir: elsewhere }\nnetworks:\n  local: { accounts: [me] }\ncontracts:\n  c:\n    type: workspace\n    source: fungible_token\n    networks: { local: }\n";
    let (_dir, loaded) = load_str(yaml, None);
    assert_eq!(codes(&loaded), vec![Code::CrateNotFound]);
}

#[test]
fn missing_file_is_a_single_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let loaded = load(dir.path(), Some(&Vec::<(String, PathBuf)>::new()), None);
    assert_eq!(codes(&loaded), vec![Code::SchemaVersion]);
}
