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

function InstallCommand() {
  const [copied, setCopied] = useState(false);
  const installCommand = 'cargo install --locked stellar-scaffold-cli';

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(installCommand);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy command:', err);
    }
  };

  return (
    <div className={styles.installCommand}>
      <div className={styles.commandText}>
        <code>{installCommand}</code>
      </div>
      <button
        className={clsx(styles.copyButton, copied && styles.copied)}
        onClick={handleCopy}
        aria-label={copied ? 'Copied!' : 'Copy installation command'}
        title={copied ? 'Copied!' : 'Copy to clipboard'}
      >
        {copied ? (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M20 6L9 17L4 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        ) : (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        )}
      </button>
    </div>
  );
}

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">
          <Translate id="homepage.tagline">
            {siteConfig.tagline}
          </Translate>
        </p>
        <InstallCommand />
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/quick-start">
            <Translate id="homepage.quickstart">Quickstart</Translate> &rarr;
          </Link>
        </div>
      </div>
    </header>
  );
}

export default function Home(): ReactNode {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title={`${siteConfig.title}`}
      description={translate({
        id: "homepage.mainMessage",
        message: "Scaffold - The Stellar app lifecycle streamlined",
        description: "The homepage message",
      })}>
      <HomepageHeader />
      <main>
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
