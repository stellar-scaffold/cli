//! Line-based edits to `scaffold.yml` that keep its comments and layout,
//! which a parse-and-reserialize round trip would drop. Every edit is checked
//! by parsing the result, so a layout these can't handle yields `None` rather
//! than a broken file.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The parts of `scaffold.yml` edits are checked against. Other keys are
/// skipped, so values serde can't represent, like huge constructor integers,
/// don't fail the check.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Probe {
    project: BTreeMap<String, String>,
    contracts: BTreeMap<String, Option<ContractProbe>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ContractProbe {
    client: Option<bool>,
}

fn probe(contents: &str) -> Option<Probe> {
    serde_saphyr::from_str(contents).ok()
}

/// Set `project.<key>: <value>`, replacing an existing entry or appending one
/// to the `project:` block, which is created after `version:` if absent.
/// `None` if the file's layout can't be edited safely.
pub fn set_project_key(contents: &str, key: &str, value: &str) -> Option<String> {
    let mut lines: Vec<String> = contents.lines().map(str::to_string).collect();
    if let Some(block) = top_level_block(&lines, "project") {
        let (start, end) = block.ok()?;
        let indent = child_indent(&lines[start + 1..end]).unwrap_or_else(|| "  ".to_string());
        let entry = format!("{indent}{key}: {value}");
        if let Some(i) = (start + 1..end).find(|&i| is_key(&lines[i], &indent, key)) {
            lines[i] = entry;
        } else {
            lines.insert(last_content_line(&lines, start, end) + 1, entry);
        }
    } else {
        let at = lines
            .iter()
            .position(|l| l.starts_with("version:"))
            .map_or(0, |i| i + 1);
        let block = [
            String::new(),
            "project:".into(),
            format!("  {key}: {value}"),
        ];
        lines.splice(at..at, block);
    }

    let out = join(&lines, contents);
    let parsed = probe(&out)?;
    (parsed.project.get(key).map(String::as_str) == Some(value)).then_some(out)
}

/// Set `client: false` on every contract under `contracts:`. Returns the file
/// unchanged when it lists no contracts, and `None` if the layout can't be
/// edited safely, e.g. a contract written inline as `name: {…}`.
pub fn disable_clients(contents: &str) -> Option<String> {
    let mut lines: Vec<String> = contents.lines().map(str::to_string).collect();
    let Some(block) = top_level_block(&lines, "contracts") else {
        return Some(contents.to_string());
    };
    let (start, end) = block.ok()?;
    let Some(contract_indent) = child_indent(&lines[start + 1..end]) else {
        return Some(contents.to_string());
    };

    let headers: Vec<usize> = (start + 1..end)
        .filter(|&i| is_entry(&lines[i], &contract_indent))
        .collect();
    // Walk backwards so inserted lines don't shift headers not yet visited.
    for (n, &header) in headers.iter().enumerate().rev() {
        let body_end = headers.get(n + 1).copied().unwrap_or(end);
        let field_indent = child_indent(&lines[header + 1..body_end])
            .filter(|i| i.len() > contract_indent.len())
            .unwrap_or_else(|| format!("{contract_indent}  "));
        let entry = format!("{field_indent}client: false");
        if let Some(i) =
            (header + 1..body_end).find(|&i| is_key(&lines[i], &field_indent, "client"))
        {
            lines[i] = entry;
        } else {
            lines.insert(header + 1, entry);
        }
    }

    let out = join(&lines, contents);
    let parsed = probe(&out)?;
    parsed
        .contracts
        .values()
        .all(|c| c.as_ref().and_then(|c| c.client) == Some(false))
        .then_some(out)
}

/// Line range of a top-level `name:` block: its header line and the exclusive
/// end, where the next top-level key starts. Comments at column 0 don't end
/// the block. `Err` when the value is written inline (`name: {…}`).
fn top_level_block(lines: &[String], name: &str) -> Option<Result<(usize, usize), ()>> {
    let header = format!("{name}:");
    let start = lines.iter().position(|l| l.starts_with(&header))?;
    let rest = lines[start][header.len()..].trim();
    if !(rest.is_empty() || rest.starts_with('#')) {
        return Some(Err(()));
    }
    let end = (start + 1..lines.len())
        .find(|&i| {
            let l = &lines[i];
            !l.is_empty() && !l.starts_with([' ', '\t']) && !l.starts_with('#')
        })
        .unwrap_or(lines.len());
    Some(Ok((start, end)))
}

/// Leading whitespace of the first entry in `lines`, skipping blanks and
/// comments.
fn child_indent(lines: &[String]) -> Option<String> {
    lines.iter().find(|l| is_content(l)).map(|l| {
        let trimmed = l.trim_start();
        l[..l.len() - trimmed.len()].to_string()
    })
}

/// Whether `line` is the `key:` entry at exactly `indent`.
fn is_key(line: &str, indent: &str, key: &str) -> bool {
    line.strip_prefix(indent)
        .and_then(|rest| rest.strip_prefix(key))
        .is_some_and(|rest| rest.starts_with(':'))
}

/// Whether `line` is a mapping entry at exactly `indent`.
fn is_entry(line: &str, indent: &str) -> bool {
    line.strip_prefix(indent)
        .is_some_and(|rest| !rest.starts_with([' ', '\t', '#']) && rest.contains(':'))
}

fn is_content(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && !trimmed.starts_with('#')
}

/// Index of the last entry line in `start..end`, or `start` if there is none,
/// so appended entries land before any trailing comments.
fn last_content_line(lines: &[String], start: usize, end: usize) -> usize {
    (start + 1..end)
        .rev()
        .find(|&i| is_content(&lines[i]))
        .unwrap_or(start)
}

/// Rejoin `lines`, keeping the original file's line endings and trailing newline.
fn join(lines: &[String], original: &str) -> String {
    let eol = if original.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = lines.join(eol);
    if original.ends_with('\n') {
        out.push_str(eol);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{disable_clients, set_project_key};

    const TEMPLATE: &str = "version: 2

# Scaffold CLI Configuration
project:
  # Your Rust/Soroban contract source.
  contracts-dir: contracts

  # Generated clients
  clients-dir: app-lib/clients

# Stellar Networks
networks:
  local:
    accounts: [me]

# Contracts
contracts:
  guess-the-number:
    type: workspace
    source: guess-the-number # the crate's package name
    networks:
      local:

  nft:
    # An NFT
    type: workspace
    source: nft
    networks:
      local:
";

    #[test]
    fn set_project_key_appends_to_existing_block() {
        let out = set_project_key(TEMPLATE, "package-manager", "pnpm").unwrap();
        assert!(
            out.contains("  clients-dir: app-lib/clients\n  package-manager: pnpm\n\n# Stellar")
        );
        assert!(out.contains("# Your Rust/Soroban contract source."));
    }

    #[test]
    fn set_project_key_replaces_existing_entry() {
        let once = set_project_key(TEMPLATE, "package-manager", "pnpm").unwrap();
        let twice = set_project_key(&once, "package-manager", "bun").unwrap();
        assert!(twice.contains("  package-manager: bun\n"));
        assert!(!twice.contains("pnpm"));
    }

    #[test]
    fn set_project_key_creates_block_after_version() {
        let out = set_project_key(
            "version: 2\nnetworks:\n  local: {}\n",
            "package-manager",
            "deno",
        )
        .unwrap();
        assert_eq!(
            out,
            "version: 2\n\nproject:\n  package-manager: deno\nnetworks:\n  local: {}\n"
        );
    }

    #[test]
    fn set_project_key_rejects_inline_project() {
        let yaml = "version: 2\nproject: { contracts-dir: contracts }\n";
        assert!(set_project_key(yaml, "package-manager", "npm").is_none());
    }

    #[test]
    fn disable_clients_marks_every_contract() {
        let out = disable_clients(TEMPLATE).unwrap();
        assert!(out.contains("  guess-the-number:\n    client: false\n    type: workspace\n"));
        assert!(out.contains("  nft:\n    client: false\n    # An NFT\n"));
        assert!(out.contains("# the crate's package name"));
    }

    #[test]
    fn disable_clients_replaces_existing_value() {
        let yaml = "version: 2\ncontracts:\n  a:\n    client: true\n    type: workspace\n";
        let out = disable_clients(yaml).unwrap();
        assert_eq!(
            out,
            "version: 2\ncontracts:\n  a:\n    client: false\n    type: workspace\n"
        );
    }

    #[test]
    fn disable_clients_rejects_inline_contracts() {
        let yaml = "version: 2\ncontracts:\n  a: { type: workspace }\n";
        assert!(disable_clients(yaml).is_none());
    }

    #[test]
    fn edits_tolerate_values_serde_cannot_represent() {
        let yaml = "version: 2\ncontracts:\n  token:\n    args:\n      initial_supply: 1000000000000000000000000\n";
        assert!(set_project_key(yaml, "package-manager", "npm").is_some());
        assert!(disable_clients(yaml).is_some());
    }

    #[test]
    fn disable_clients_without_contracts_is_unchanged() {
        let yaml = "version: 2\nnetworks:\n  local: {}\n";
        assert_eq!(disable_clients(yaml).unwrap(), yaml);
    }
}
