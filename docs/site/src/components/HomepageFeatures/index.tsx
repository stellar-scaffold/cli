import type {ReactNode} from 'react';
import Link from '@docusaurus/Link';
import {translate} from '@docusaurus/Translate';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  to: string;
  description: string;
  cta: string;
};

/**
 * The four entry points into the documentation, ordered the way a reader
 * arrives at them: install, follow along, look things up, then ship.
 */
const FeatureList: FeatureItem[] = [
  {
    title: translate({
      id: 'homepage.features.quickstart.title',
      message: 'Quickstart',
      description: 'Title for the quickstart card',
    }),
    to: '/docs/quick-start',
    description: translate({
      id: 'homepage.features.quickstart.description',
      message:
        'Install the CLI, create a project, and get a contract and frontend running against a local network in a few minutes.',
      description: 'Description for the quickstart card',
    }),
    cta: translate({
      id: 'homepage.features.quickstart.cta',
      message: 'Start here',
      description: 'Link label on the quickstart card',
    }),
  },
  {
    title: translate({
      id: 'homepage.features.tutorial.title',
      message: 'Tutorial',
      description: 'Title for the tutorial card',
    }),
    to: '/docs/tutorial/overview',
    description: translate({
      id: 'homepage.features.tutorial.description',
      message:
        'Build a complete dApp end to end — write the contract, wire up the generated client, add payments, and harden it.',
      description: 'Description for the tutorial card',
    }),
    cta: translate({
      id: 'homepage.features.tutorial.cta',
      message: 'Build a dApp',
      description: 'Link label on the tutorial card',
    }),
  },
  {
    title: translate({
      id: 'homepage.features.cli.title',
      message: 'CLI reference',
      description: 'Title for the CLI card',
    }),
    to: '/docs/cli',
    description: translate({
      id: 'homepage.features.cli.description',
      message:
        'Every command in detail: init, build, generate, watch, and upgrade, plus the configuration each one reads.',
      description: 'Description for the CLI card',
    }),
    cta: translate({
      id: 'homepage.features.cli.cta',
      message: 'Look up a command',
      description: 'Link label on the CLI card',
    }),
  },
  {
    title: translate({
      id: 'homepage.features.registry.title',
      message: 'Registry',
      description: 'Title for the registry card',
    }),
    to: '/docs/registry',
    description: translate({
      id: 'homepage.features.registry.description',
      message:
        'Publish a WASM, deploy named contract instances, and reuse contracts other people have already published.',
      description: 'Description for the registry card',
    }),
    cta: translate({
      id: 'homepage.features.registry.cta',
      message: 'Publish and deploy',
      description: 'Link label on the registry card',
    }),
  },
];

function Feature({title, to, description, cta}: FeatureItem) {
  return (
    <Link to={to} className={styles.card}>
      <Heading as="h2" className={styles.cardTitle}>
        {title}
      </Heading>
      <p className={styles.cardDesc}>{description}</p>
      <span className={styles.cardLink}>{cta} &rarr;</span>
    </Link>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.features}>
      {FeatureList.map((props) => (
        <Feature key={props.to} {...props} />
      ))}
    </section>
  );
}
