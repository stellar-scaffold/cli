---
sidebar_label: AI Agent Skills
---

# AI Agent Skills

Stellar Scaffold publishes an [Agent Skill](https://agentskills.io): a folder of Markdown instructions that teaches AI coding agents (Claude Code, Codex, Cursor, and others) how to create and work in Scaffold projects. With it loaded, an agent knows the `stellar scaffold` commands, writes valid `scaffold.yml`, calls your contracts through the generated clients instead of hardcoding IDs, and avoids the common mistakes that come from outdated examples online.

## What's in the skill

The skill lives in the CLI repository at [`skills/stellar-scaffold/`](https://github.com/stellar-scaffold/cli/tree/main/skills/stellar-scaffold), versioned alongside the CLI it describes.

| File | Covers |
| --- | --- |
| `SKILL.md` | Setup, `init` and `upgrade`, project layout, the dev loop, `scaffold.yml` essentials, calling contracts from your app, adding contracts, testnet and mainnet, diagnostics, and common pitfalls |
| `references/cli.md` | Every `stellar scaffold` command and flag |
| `references/config.md` | The full `scaffold.yml` schema, `.env`, contract metadata, and migrating from `environments.toml` |
| `references/frontend.md` | Generated clients, `@stellar-scaffold/app-lib`, wallets, and keeping the app and CLI on the same network |

Agents read `SKILL.md` first and open a reference file only when a task needs it, so the skill costs little context until it's used.

## Using the skill

### Point your agent at it

Any agent that can fetch a URL can use the skill directly. Start your session with:

```text
Read https://raw.githubusercontent.com/stellar-scaffold/cli/main/skills/stellar-scaffold/SKILL.md
and follow it while you work on this project.
```

The agent follows the relative links in `SKILL.md` to the reference files when it needs them. This always gets the latest version, and needs no setup.

### Install it for Claude Code

Claude Code loads skills automatically from `~/.claude/skills/` (all your projects) or `.claude/skills/` (one project), and decides when to use them from the skill's description. Copy the skill folder into one of those:

```bash
git clone --depth 1 https://github.com/stellar-scaffold/cli.git /tmp/stellar-scaffold-cli
mkdir -p ~/.claude/skills
cp -R /tmp/stellar-scaffold-cli/skills/stellar-scaffold ~/.claude/skills/
```

Then ask for Scaffold work in plain language, such as "create a Scaffold project with a Svelte frontend" or "add a token contract and call it from the app", and Claude Code loads the skill when it's relevant.

### Other agents

Agents that support the Agent Skills format load them from their own skills directory. Copy the same `skills/stellar-scaffold` folder there; check your agent's documentation for the location. For agents without skill support, use the URL prompt above, or add it to your project's `AGENTS.md` so every session starts with it:

```markdown
## Stellar Scaffold

This project uses Stellar Scaffold. Before changing contracts, `scaffold.yml`, or contract calls in the app, read https://raw.githubusercontent.com/stellar-scaffold/cli/main/skills/stellar-scaffold/SKILL.md
```

### Keeping it current

The skill tracks the CLI's `main` branch, which can be slightly ahead of the latest release. An installed copy doesn't update itself, so re-copy it when you upgrade `stellar-scaffold-cli`, or use the URL approach to always read the current version.

## Related skills

The Scaffold skill covers Scaffold itself and links out for neighboring tasks:

- **Stellar Registry**: publishing and deploying contracts by name, and calling registry contracts from Rust. Skill: https://raw.githubusercontent.com/stellar-registry/cli/main/skills/stellar-registry/SKILL.md
- **Stellar development in general**: [stellar/stellar-dev-skill](https://github.com/stellar/stellar-dev-skill) indexes skills for the wider Stellar ecosystem, including Soroban, the SDKs, and wallets.

## Feedback

If an agent gets something wrong while using the skill, [open an issue](https://github.com/stellar-scaffold/cli/issues) with the prompt and what it did. Fixes reach the URL as soon as they merge, and installed copies the next time you re-copy the folder.
