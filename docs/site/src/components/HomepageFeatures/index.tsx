import type {ReactNode} from 'react';
import clsx from 'clsx';
import Heading from '@theme/Heading';
import {translate} from '@docusaurus/Translate';
import Translate from '@docusaurus/Translate';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  Svg: React.ComponentType<React.ComponentProps<'svg'>>;
  description: ReactNode;
};

const FeatureList: FeatureItem[] = [
  {
    title: translate({
      id: 'homepage.features.stellar.title',
      message: 'Build with Stellar Smart Contracts',
      description: 'Title for the first feature card',
    }),
    Svg: require('@site/static/img/stellar.svg').default,
    description: (
      <Translate id="homepage.features.stellar.description">
        Simple and power Stellar Smart Contract management.
      </Translate>
    ),
  },
  {
    title: translate({
      id: 'homepage.features.tooling.title',
      message: 'Smart Contract Tooling',
      description: 'Title for the second feature card',
    }),
    Svg: require('@site/static/img/tooling.svg').default,
    description: (
      <Translate id="homepage.features.tooling.description">
        Use standard-redefining tools at all levels of the Stellar software stack, making it easier to build, test, and ship dapps.
      </Translate>
    ),
  },
  {
    title: translate({
      id: 'homepage.features.practices.title',
      message: 'Best Practices',
      description: 'Title for the third feature card',
    }),
    Svg: require('@site/static/img/code_hero.svg').default,
    description: (
      <Translate id="homepage.features.practices.description">
        Write beautiful, maintainable and secure code from the get go.
      </Translate>
    ),
  }
];

function Feature({title, Svg, description}: FeatureItem) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center">
        <Svg className={styles.featureSvg} role="img" />
      </div>
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
