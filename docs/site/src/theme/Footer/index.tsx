import type {ReactNode} from 'react';
import Link from '@docusaurus/Link';
import Translate from '@docusaurus/Translate';

import styles from './styles.module.css';

/**
 * Custom site footer, in two parts:
 *
 *  1. A brand band promoting the sibling Stellar Registry, mirroring the band
 *     the Registry runs to promote Stellar Scaffold.
 *  2. The footer proper: site navigation, Stellar ecosystem links, and the
 *     studio credit, over a copyright bar.
 *
 * This replaces the classic theme's config-driven footer, so `themeConfig.footer`
 * is intentionally absent from docusaurus.config.ts.
 */

function AhaLogo() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 130 45"
      fill="none"
      className={styles.brandLogo}
      aria-hidden="true"
      focusable="false">
      <g clipPath="url(#aha-footer-clip)">
        <path
          d="M4.5895 22.0971C3.80747 21.3129 2.37745 19.6408 0.00976562 18.876V28.7521C1.78207 27.0507 3.31954 24.6409 4.5895 22.0966V22.0971Z"
          fill="currentColor"
        />
        <path
          d="M31.9084 31.2354C31.1341 31.8264 30.1671 32.1758 29.1203 32.1758C28.1993 32.1758 27.3453 31.9056 26.622 31.4353C25.9972 31.9128 25.2301 32.1758 24.4105 32.1758C22.3457 32.1758 20.6626 30.4949 20.6626 28.428V25.6349C20.6626 24.6053 19.823 23.7657 18.7934 23.7657C17.7638 23.7657 16.9242 24.6048 16.9242 25.6349V31.9997H14.1793V22.6348C13.7429 22.9095 13.3109 23.1437 12.8839 23.3393V32.0003H10.1418V24.0703C9.05902 24.1501 7.99065 23.9712 6.94112 23.5325C5.20482 26.9631 2.90581 30.3265 0.00976562 32.2656V36.9023H36.7623V31.9969L33.5849 31.9958C32.9169 31.9958 32.3188 31.7017 31.9084 31.236V31.2354Z"
          fill="currentColor"
        />
        <path
          d="M18.1805 12.168C17.8548 12.1802 16.9238 12.7418 16.9238 15.0912V16.5793C18.998 14.1651 18.8916 12.168 18.1805 12.168Z"
          fill="currentColor"
        />
        <path
          d="M8.28696 12.1683H12.8839V20.2168C13.2965 19.9493 13.7279 19.6308 14.1793 19.2592V15.0915C14.1793 10.6126 16.7984 9.42845 18.1803 9.42845C19.4608 9.42845 20.5729 10.1905 21.0841 11.4184C21.5205 12.4746 22.0827 15.3784 17.8934 19.5704C17.5605 19.9011 17.2398 20.2118 16.9236 20.5014V21.4253C17.493 21.1694 18.1249 21.027 18.7928 21.027C21.3322 21.027 23.3991 23.094 23.3991 25.6361V28.4292C23.4135 29.2948 24.323 29.6327 24.858 29.3242C24.6314 28.7842 24.5107 28.1905 24.5107 27.5685V25.6367C24.5107 23.0945 26.5777 21.0276 29.1198 21.0276C29.9328 21.0276 30.6971 21.2375 31.3606 21.6113V21.0132H33.8856V28.1666C33.8856 29.0024 34.4505 29.2594 34.8067 29.2594H36.7617V0.149902H0.00976562V16.0818C2.0324 16.5968 4.02679 17.7139 5.78359 19.5023C6.95109 16.7552 7.79515 14.078 8.28696 12.1678V12.1683Z"
          fill="currentColor"
        />
        <path
          d="M10.1419 21.3196V15.5796C9.65232 17.1209 8.97497 19.0406 8.10156 21.0521C8.90962 21.3678 9.59196 21.3922 10.1419 21.3196Z"
          fill="currentColor"
        />
        <path
          d="M29.121 23.7656C28.0886 23.7656 27.2495 24.6047 27.2495 25.6348V27.5667C27.2495 28.5963 28.0886 29.4359 29.121 29.4359C30.1533 29.4359 30.9902 28.5963 30.9902 27.5667V25.6348C30.9902 24.6053 30.1533 23.7656 29.121 23.7656Z"
          fill="currentColor"
        />
        <path
          d="M5.16605 44.7302H36.7623V38.1201H0.00976562L5.16605 44.7302Z"
          fill="currentColor"
        />
        <path
          d="M48.6092 19.7812V8.90208H44.7622V6.89551H54.6655V8.90208H50.8185V19.7812H48.6098H48.6092Z"
          fill="currentColor"
        />
        <path
          d="M56.3955 19.7812V6.89551H58.6231V12.3071H63.7035V6.89551H65.931V19.7812H63.7035V14.3325H58.6231V19.7812H56.3955Z"
          fill="currentColor"
        />
        <path
          d="M68.4713 19.7812V6.89551H76.6809V8.90208H70.6983V12.2888H76.1653V14.2954H70.6983V17.7747H76.7728V19.7812H68.4707H68.4713Z"
          fill="currentColor"
        />
        <path
          d="M82.2769 19.7812L85.811 6.89551H89.658L93.1921 19.7812H90.9097L90.1365 16.8359H85.3319L84.5588 19.7812H82.2764H82.2769ZM85.8481 14.811H89.6214L87.8912 8.27624H87.5783L85.8481 14.811Z"
          fill="currentColor"
        />
        <path
          d="M94.8311 19.7812V6.89551H97.0586V12.3071H102.139V6.89551H104.367V19.7812H102.139V14.3325H97.0586V19.7812H94.8311Z"
          fill="currentColor"
        />
        <path
          d="M106.005 19.7812L109.539 6.89551H113.386L116.92 19.7812H114.638L113.865 16.8359H109.06L108.287 19.7812H106.004H106.005ZM109.576 14.811H113.349L111.619 8.27624H111.306L109.576 14.811Z"
          fill="currentColor"
        />
        <path
          d="M50.4133 37.9849C48.8421 37.9849 47.5904 37.5401 46.6583 36.6501C45.7256 35.7606 45.2593 34.4812 45.2593 32.812V29.7564C45.2593 28.0877 45.7256 26.8083 46.6583 25.9183C47.591 25.0288 48.8426 24.5835 50.4133 24.5835C51.984 24.5835 53.1959 25.0133 54.0488 25.8723C54.9017 26.7313 55.3282 27.9093 55.3282 29.4064V29.5166H53.156V29.351C53.156 28.5413 52.9317 27.8755 52.4842 27.3538C52.0361 26.8327 51.346 26.5718 50.4133 26.5718C49.4807 26.5718 48.79 26.8509 48.2689 27.4092C47.7471 27.968 47.4863 28.7379 47.4863 29.7193V32.8485C47.4863 33.8183 47.7471 34.5854 48.2689 35.1497C48.7906 35.7141 49.505 35.9966 50.4133 35.9966C51.3216 35.9966 52.0361 35.7357 52.4842 35.214C52.9322 34.6928 53.156 34.0265 53.156 33.2168V32.9039H55.3282V33.1614C55.3282 34.659 54.9017 35.837 54.0488 36.6955C53.1959 37.5545 51.984 37.9843 50.4133 37.9843V37.9849Z"
          fill="currentColor"
        />
        <path
          d="M62.3413 37.9849C60.7457 37.9849 59.4724 37.5401 58.5214 36.6501C57.5705 35.7606 57.0947 34.4812 57.0947 32.812V29.7564C57.0947 28.0877 57.5699 26.8083 58.5214 25.9183C59.4724 25.0288 60.7457 24.5835 62.3413 24.5835C63.9369 24.5835 65.2252 25.0282 66.17 25.9183C67.1149 26.8083 67.5873 28.0877 67.5873 29.7564V32.812C67.5873 34.4812 67.1149 35.7606 66.17 36.6501C65.2252 37.5401 63.9485 37.9849 62.3413 37.9849ZM62.3413 35.9971C63.2983 35.9971 64.0438 35.718 64.5777 35.1597C65.1116 34.6014 65.3786 33.8438 65.3786 32.8862V29.6833C65.3786 28.7262 65.1116 27.9686 64.5777 27.4098C64.0438 26.8515 63.2983 26.5723 62.3413 26.5723C61.3842 26.5723 60.6571 26.8515 60.1231 27.4098C59.5892 27.9686 59.3223 28.7257 59.3223 29.6833V32.8862C59.3223 33.8432 59.5892 34.6014 60.1231 35.1597C60.6571 35.7185 61.3964 35.9971 62.3413 35.9971Z"
          fill="currentColor"
        />
        <path
          d="M69.8154 37.7275V24.8418H73.9571L76.4604 36.236H76.7734L79.295 24.8418H83.4184V37.7275H81.2833V26.4435H80.9704L78.467 37.7275H74.7856L72.2822 26.4435H71.9693V37.7275H69.8154Z"
          fill="currentColor"
        />
        <path
          d="M85.9771 37.7275V24.8418H91.1865C91.9962 24.8418 92.7079 25.0013 93.3216 25.3203C93.9352 25.6399 94.4198 26.0907 94.776 26.6734C95.1315 27.2566 95.3099 27.9466 95.3099 28.7442V29.0017C95.3099 29.7871 95.1288 30.4744 94.7671 31.0631C94.4049 31.6519 93.9142 32.1066 93.2944 32.425C92.6747 32.7446 91.9719 32.9035 91.1865 32.9035H88.2046V37.7264H85.9771V37.7275ZM88.2046 30.8981H90.9655C91.6036 30.8981 92.1159 30.7264 92.5024 30.383C92.889 30.0396 93.0823 29.5672 93.0823 28.9657V28.7818C93.0823 28.1804 92.8918 27.7079 92.5119 27.3646C92.1314 27.0212 91.6157 26.8495 90.9655 26.8495H88.2046V30.8992V30.8981Z"
          fill="currentColor"
        />
        <path
          d="M95.1632 37.7275L98.6972 24.8418H102.544L106.078 37.7275H103.796L103.023 34.7822H98.2182L97.445 37.7275H95.1626H95.1632ZM98.7338 32.7573H102.507L100.777 26.2225H100.464L98.7338 32.7573Z"
          fill="currentColor"
        />
        <path
          d="M107.717 37.7275V24.8418H111.95L114.988 36.236H115.282V24.8418H117.473V37.7275H113.257L110.22 26.3333H109.907V37.7275H107.717Z"
          fill="currentColor"
        />
        <path
          d="M123.345 37.727V33.1805L118.909 24.8418H121.394L124.303 30.5298H124.615L127.524 24.8418H129.99L125.554 33.1805V37.727H123.345Z"
          fill="currentColor"
        />
      </g>
      <defs>
        <clipPath id="aha-footer-clip">
          <rect
            width="129.981"
            height="44.5803"
            fill="currentColor"
            transform="translate(0.00976562 0.149902)"
          />
        </clipPath>
      </defs>
    </svg>
  );
}

function RegistryBanner() {
  return (
    <section className={styles.banner}>
      <div className={styles.bannerInner}>
        <div className={styles.bannerText}>
          <p className={styles.bannerEyebrow}>
            <Translate id="footer.banner.eyebrow">Contract Registry</Translate>
          </p>
          <h2 className={styles.bannerTitle}>
            <Translate id="footer.banner.title">
              Publish and deploy your contracts with Stellar Registry
            </Translate>
          </h2>
          <p className={styles.bannerDesc}>
            <Translate id="footer.banner.description">
              Publish a WASM once, deploy named instances from it, and let other
              people build on your contracts by name instead of by address.
            </Translate>
          </p>
        </div>
        <Link
          href="https://rgstry.xyz"
          className={styles.bannerCta}
          target="_blank"
          rel="noopener noreferrer">
          <Translate id="footer.banner.cta">Browse the registry</Translate>{' '}
          &rarr;
        </Link>
      </div>
    </section>
  );
}

export default function Footer(): ReactNode {
  return (
    <>
      <RegistryBanner />
      <footer className={styles.footer}>
        <div className={styles.footerInner}>
          <div className={styles.col}>
            <p className={styles.colTitle}>
              <Translate id="footer.col.scaffold">Stellar Scaffold</Translate>
            </p>
            <Link to="/docs/quick-start" className={styles.link}>
              <Translate id="footer.link.quickstart">Quickstart</Translate>
            </Link>
            <Link to="/docs/tutorial/overview" className={styles.link}>
              <Translate id="footer.link.tutorial">Tutorial</Translate>
            </Link>
            <Link to="/docs/cli" className={styles.link}>
              <Translate id="footer.link.cli">CLI reference</Translate>
            </Link>
            <Link to="/blog" className={styles.link}>
              <Translate id="footer.link.blog">Blog</Translate>
            </Link>
            <Link to="/showcase" className={styles.link}>
              <Translate id="footer.link.showcase">Showcase</Translate>
            </Link>
            <Link href="https://github.com/stellar-scaffold/cli" className={styles.link}>
              <Translate id="footer.link.github">GitHub</Translate>
            </Link>
          </div>

          <div className={styles.col}>
            <p className={styles.colTitle}>
              <Translate id="footer.col.stellar">Stellar</Translate>
            </p>
            <Link
              href="https://developers.stellar.org/docs/smart-contracts"
              className={styles.link}>
              <Translate id="footer.link.smartContracts">
                Smart Contracts
              </Translate>
            </Link>
            <Link href="https://developers.stellar.org/" className={styles.link}>
              <Translate id="footer.link.developerDocs">Developer Docs</Translate>
            </Link>
            <Link href="https://rgstry.xyz" className={styles.link}>
              <Translate id="footer.link.registry">Stellar Registry</Translate>
            </Link>
            <Link href="https://stellar.expert/" className={styles.link}>
              <Translate id="footer.link.expert">Stellar Expert</Translate>
            </Link>
          </div>

          <div className={styles.col}>
            <p className={styles.colTitle}>
              <Translate id="footer.col.builtBy">Built by</Translate>
            </p>
            <Link
              href="https://www.theaha.co/"
              className={styles.brand}
              aria-label="The Aha Company">
              <AhaLogo />
            </Link>
            <p className={styles.brandDesc}>
              <Translate id="footer.brand.description">
                A software studio building tools for developers and the teams
                that support them.
              </Translate>
            </p>
          </div>
        </div>

        <div className={styles.footerBottom}>
          <p className={styles.copyright}>
            &copy; {new Date().getFullYear()}{' '}
            <Link href="https://www.theaha.co/" className={styles.copyrightLink}>
              The Aha Company
            </Link>
          </p>
          <p className={styles.builtWith}>
            <Translate id="footer.builtWith">Built with</Translate>{' '}
            <Link href="https://stellar.org" className={styles.copyrightLink}>
              Stellar
            </Link>
          </p>
        </div>
      </footer>
    </>
  );
}
