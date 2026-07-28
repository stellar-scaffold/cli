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

/**
 * The brand mark: a braced frame, the shape of the scaffolding that holds a
 * structure while it is being built. Reused as the favicon/logo motif.
 */
function ScaffoldMark({className}: {className?: string}) {
  return (
    <svg
      className={className}
      viewBox="0 0 32 32"
      fill="none"
      role="img"
      aria-hidden="true"
      focusable="false">
      <rect
        x="3.5"
        y="3.5"
        width="25"
        height="25"
        rx="1.5"
        stroke="currentColor"
        strokeWidth="2"
      />
      <path
        d="M3.5 16h25M16 3.5v25M3.5 3.5l25 25M28.5 3.5l-25 25"
        stroke="currentColor"
        strokeWidth="1.25"
        opacity="0.45"
      />
    </svg>
  );
}

const INSTALL_COMMAND = 'cargo install --locked stellar-scaffold-cli';

function InstallCommand() {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(INSTALL_COMMAND);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy command:', err);
    }
  };

  return (
    <div className={styles.installCommand}>
      <code className={styles.commandText}>{INSTALL_COMMAND}</code>
      <button
        className={clsx(styles.copyButton, copied && styles.copied)}
        onClick={handleCopy}
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
        {copied ? (
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
        )}
      </button>
    </div>
  );
}

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={styles.hero}>
      <div className={styles.heroInner}>
        <ScaffoldMark className={styles.heroMark} />
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
      </div>
    </header>
  );
}

type LoopStep = {
  command: string;
  detail: string;
};

/**
 * The four commands that make up a working session, in the order they run.
 * The sequence is the product, so the page renders it as one.
 */
function DevLoop() {
  const steps: LoopStep[] = [
    {
      command: 'stellar scaffold init my-project',
      detail: translate({
        id: 'homepage.loop.init',
        message:
          'Creates a project from the template monorepo: contracts, a frontend, and the shared app library, already wired together.',
        description: 'Description of the init step',
      }),
    },
    {
      command: 'stellar scaffold generate contract counter',
      detail: translate({
        id: 'homepage.loop.generate',
        message:
          'Adds a contract to the workspace and registers it in your environment config.',
        description: 'Description of the generate step',
      }),
    },
    {
      command: 'npm run dev',
      detail: translate({
        id: 'homepage.loop.dev',
        message:
          'Watches your contracts, rebuilds and redeploys them on change, regenerates typed clients, and serves the frontend.',
        description: 'Description of the dev step',
      }),
    },
    {
      command: 'stellar registry publish',
      detail: translate({
        id: 'homepage.loop.publish',
        message:
          'Publishes your WASM to the Stellar Registry so it can be deployed by name on testnet or mainnet.',
        description: 'Description of the publish step',
      }),
    },
  ];

  return (
    <section className={styles.loop}>
      <Heading as="h2" className={styles.sectionHeading}>
        <Translate id="homepage.loop.heading">The loop</Translate>
      </Heading>
      <p className={styles.sectionBody}>
        <Translate id="homepage.loop.intro">
          Four commands cover the whole lifecycle. Everything in between —
          building contracts, deploying to a local network, generating
          TypeScript clients — happens on its own.
        </Translate>
      </p>
      <ol className={styles.loopList}>
        {steps.map((step) => (
          <li key={step.command} className={styles.loopStep}>
            <code className={styles.loopCommand}>{step.command}</code>
            <p className={styles.loopDetail}>{step.detail}</p>
          </li>
        ))}
      </ol>
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

        <DevLoop />
      </main>
    </Layout>
  );
}
