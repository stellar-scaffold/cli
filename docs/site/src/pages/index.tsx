import type {ReactNode} from 'react';
import {useState} from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import {translate} from '@docusaurus/Translate';
import Translate from '@docusaurus/Translate';
import Layout from '@theme/Layout';
import HomepageFeatures from '@site/src/components/HomepageFeatures';
import Heading from '@theme/Heading';

import styles from './index.module.css';

const INSTALL_COMMAND = 'cargo binstall stellar-scaffold-cli';

/**
 * Copy-paste prompt that walks an AI coding agent through the Quickstart and
 * hands it the Agent Skill, which covers everything after `init`.
 */
const AGENT_PROMPT = `Help me start a new Stellar Scaffold project.

1. Read the Stellar Scaffold agent skill and follow it for everything below: https://raw.githubusercontent.com/stellar-scaffold/cli/main/skills/stellar-scaffold/SKILL.md
2. Check that I have Rust with the wasm32v1-none target, a C toolchain, the Stellar CLI (\`stellar\`), Node.js, and Docker running. If anything is missing, tell me how to install it before going further.
3. Install the Scaffold CLI with \`cargo binstall stellar-scaffold-cli\`, installing cargo-binstall first if I don't have it (see https://stellarscaffold.org/docs/quick-start). Confirm it works with \`stellar scaffold version\`.
4. Ask me for a project name, and whether I want a React or Svelte frontend, or contracts only.
5. Run \`stellar scaffold init <name> --template <react|svelte|none> -y\`. The first run downloads the local network's Docker image and deploys the example contracts, so it can take several minutes.
6. For a frontend project, start \`npm run dev\` in the project. It keeps running, so run it in the background or ask me to run it, then point me to http://localhost:5173.

Then ask me what I'd like to build.`;

/** Anchor the hero links to so small screens can jump past the cards. */
const AGENT_PROMPT_ID = 'ai-prompt';

/**
 * Copies `text` to the clipboard and reports `copied` for two seconds after,
 * so the button can confirm it worked.
 */
function useCopy(text: string): [boolean, () => Promise<void>] {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return [copied, copy];
}

/** A check mark once copied, otherwise the two-sheets copy icon. */
function CopyIcon({copied}: {copied: boolean}) {
  return copied ? (
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M20 6L9 17L4 12"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  ) : (
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <rect
        x="9"
        y="9"
        width="13"
        height="13"
        rx="2"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function InstallCommand() {
  const [copied, copy] = useCopy(INSTALL_COMMAND);

  return (
    <div className={styles.installCommand}>
      <code className={styles.commandText}>{INSTALL_COMMAND}</code>
      <button
        className={clsx(styles.copyButton, copied && styles.copied)}
        onClick={copy}
        aria-label={
          copied
            ? translate({
                id: 'homepage.install.copied',
                message: 'Command copied',
                description: 'Accessible label after copying the install command',
              })
            : translate({
                id: 'homepage.install.copy',
                message: 'Copy install command',
                description: 'Accessible label for the copy button',
              })
        }>
        <CopyIcon copied={copied} />
      </button>
    </div>
  );
}

/**
 * Callout with the agent prompt. Set apart in the primary color because it's
 * the one block on the page meant to be copied whole.
 */
function AgentPrompt() {
  const [copied, copy] = useCopy(AGENT_PROMPT);

  return (
    <section id={AGENT_PROMPT_ID} className={styles.agent}>
      <div className={styles.agentHeader}>
        <Heading as="h2" className={styles.agentHeading}>
          <Translate id="homepage.agent.heading">
            Building with an AI agent?
          </Translate>
        </Heading>
        <button
          className={clsx(styles.agentCopy, copied && styles.copied)}
          onClick={copy}>
          <CopyIcon copied={copied} />
          {copied ? (
            <Translate id="homepage.agent.copied">Copied</Translate>
          ) : (
            <Translate id="homepage.agent.copy">Copy prompt</Translate>
          )}
        </button>
      </div>
      <p className={styles.agentBody}>
        <Translate id="homepage.agent.body">
          Paste this into Claude Code, Codex, Cursor, or another coding agent.
          It installs Scaffold, asks for your project name, creates the
          project, and points the agent at our Agent Skill for everything
          after that.
        </Translate>
      </p>
      <pre className={styles.agentPrompt}>{AGENT_PROMPT}</pre>
      <Link className={styles.sectionLink} to="/docs/agent-skills">
        <Translate id="homepage.agent.cta">About the Agent Skill</Translate>{' '}
        &rarr;
      </Link>
    </section>
  );
}

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={styles.hero}>
      <div className={styles.heroInner}>
        <Heading as="h1" className={styles.heroHeading}>
          {siteConfig.title}
        </Heading>
        <p className={styles.heroSub}>
          <Translate id="homepage.tagline">
            Go from an empty directory to a deployed Stellar dApp without
            wiring the pieces together yourself.
          </Translate>
        </p>
        <InstallCommand />
        <Link className={styles.heroCta} to="/docs/quick-start">
          <Translate id="homepage.quickstart">Quickstart</Translate> &rarr;
        </Link>
        <a className={styles.heroAgentLink} href={`#${AGENT_PROMPT_ID}`}>
          <Translate id="homepage.agent.jump">
            Using AI? Copy our prompt to get started.
          </Translate>
        </a>
      </div>
    </header>
  );
}

type WorkflowStep = {
  label: string;
  command: string;
  detail: string;
};

/**
 * The three stages of a project, in order: fast iteration against a local
 * network, then deliberate deploys to testnet and mainnet. Each stage is
 * labeled because the command alone doesn't say which stage it belongs to.
 */
function Workflow() {
  const steps: WorkflowStep[] = [
    {
      label: translate({
        id: 'homepage.workflow.init.label',
        message: 'Create',
        description: 'Label for the init step',
      }),
      command: 'stellar scaffold init my-project',
      detail: translate({
        id: 'homepage.workflow.init',
        message:
          'Sets up contracts, a frontend in the framework you pick, and the utility code connecting them, already wired together.',
        description: 'Description of the init step',
      }),
    },
    {
      label: translate({
        id: 'homepage.workflow.dev.label',
        message: 'Develop locally',
        description: 'Label for the dev step',
      }),
      command: 'npm run dev',
      detail: translate({
        id: 'homepage.workflow.dev',
        message:
          'Runs a local network and watches your code. Every save rebuilds and redeploys the contracts that changed, regenerates their typed clients, and reloads the frontend.',
        description: 'Description of the dev step',
      }),
    },
    {
      label: translate({
        id: 'homepage.workflow.publish.label',
        message: 'Ship',
        description: 'Label for the publish step',
      }),
      command: 'stellar registry publish',
      detail: translate({
        id: 'homepage.workflow.publish',
        message:
          'When a contract is ready, publish its Wasm to the Stellar Registry and deploy it by name: to testnet for staging, then to mainnet. These deploys are deliberate steps, never a side effect of saving a file.',
        description: 'Description of the publish step',
      }),
    },
  ];

  return (
    <section className={styles.workflow}>
      <Heading as="h2" className={styles.sectionHeading}>
        <Translate id="homepage.workflow.heading">The workflow</Translate>
      </Heading>
      <p className={styles.sectionBody}>
        <Translate id="homepage.workflow.intro">
          Iterate locally as fast as you can save, then deploy to testnet and
          mainnet when you mean to. Building contracts, running the local
          network, and generating TypeScript clients happen on their own.
        </Translate>
      </p>
      <ol className={styles.workflowList}>
        {steps.map((step) => (
          <li key={step.command} className={styles.workflowStep}>
            <span className={styles.workflowLabel}>{step.label}</span>
            <code className={styles.workflowCommand}>{step.command}</code>
            <p className={styles.workflowDetail}>{step.detail}</p>
          </li>
        ))}
      </ol>
      <p className={clsx(styles.sectionBody, styles.workflowExtras)}>
        <Translate id="homepage.workflow.extras">
          While you develop, one committed scaffold.yml declares your networks,
          accounts, and contracts, so everyone on your team builds the same
          project, and the same file carries it from local to testnet to
          mainnet. Config checks catch mistakes before a build runs, and when
          something does break, Doctor checks your toolchain, config, and
          local network in one pass and tells you how to fix what it finds.
        </Translate>
      </p>
      <Link className={styles.sectionLink} to="/docs/cli">
        <Translate id="homepage.workflow.cta">See every command</Translate>{' '}
        &rarr;
      </Link>
    </section>
  );
}

export default function Home(): ReactNode {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title={siteConfig.title}
      description={translate({
        id: 'homepage.mainMessage',
        message:
          'Stellar Scaffold — a developer toolkit for building smart contracts and dApps on Stellar',
        description: 'The homepage message',
      })}>
      <HomepageHeader />
      <main className={styles.main}>
        <HomepageFeatures />

        <AgentPrompt />

        <section className={styles.about}>
          <Heading as="h2" className={styles.sectionHeading}>
            <Translate id="homepage.about.heading">
              What is Stellar Scaffold?
            </Translate>
          </Heading>
          <p className={styles.sectionBody}>
            <Translate id="homepage.about.p1">
              Stellar Scaffold is a plugin for the Stellar CLI that scaffolds
              and maintains full-stack Soroban projects. It generates the Rust
              contract workspace, a frontend template, and the typed TypeScript
              clients that connect the two — then keeps the clients in sync as
              your contracts change.
            </Translate>
          </p>
          <p className={styles.sectionBody}>
            <Translate id="homepage.about.p2">
              It is built for the part of the work that is not your idea: local
              network setup, deploy addresses, client regeneration, environment
              config. You write contracts and UI; the toolkit keeps the wiring
              correct.
            </Translate>
          </p>
          <Link className={styles.sectionLink} to="/docs/intro">
            <Translate id="homepage.about.cta">Read the introduction</Translate>{' '}
            &rarr;
          </Link>
        </section>

        <Workflow />
      </main>
    </Layout>
  );
}
