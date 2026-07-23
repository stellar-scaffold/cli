# Changelog

All notable changes to this project will be documented in this file. Do not edit manually.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.26](https://github.com/stellar-scaffold/cli/compare/stellar-scaffold-cli-v0.0.25...stellar-scaffold-cli-v0.0.26) - 2026-07-21

### Added

- allow --no-template option on init ([#564](https://github.com/stellar-scaffold/cli/pull/564))
- optimize build ([#569](https://github.com/stellar-scaffold/cli/pull/569))

## [0.0.25](https://github.com/stellar-scaffold/cli/compare/stellar-scaffold-cli-v0.0.24...stellar-scaffold-cli-v0.0.25) - 2026-06-29

### Added

- upgrade to stellar protocol 27 ([#549](https://github.com/stellar-scaffold/cli/pull/549))
- add version flag ([#492](https://github.com/stellar-scaffold/cli/pull/492))

### Other

- Template Monorepo ([#543](https://github.com/stellar-scaffold/cli/pull/543))
- extract scaffold-cli from monorepo ([#535](https://github.com/stellar-scaffold/cli/pull/535))
- Feat/pkg manager improvements ([#491](https://github.com/stellar-scaffold/cli/pull/491))

## [0.0.24](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.23...stellar-scaffold-cli-v0.0.24) - 2026-04-27

### Fixed

- align deps and struct shapes for stellar-cli 26.0.0 ([#486](https://github.com/theahaco/scaffold-stellar/pull/486))
- versions and missing changelog ([#477](https://github.com/theahaco/scaffold-stellar/pull/477))

### Other

- make stellar scaffold package manager agnostic ([#345](https://github.com/theahaco/scaffold-stellar/pull/345))

## [0.0.23](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.22...stellar-scaffold-cli-v0.0.23) - 2026-04-16

### Other

- Feat/reporter extension ([#463](https://github.com/theahaco/scaffold-stellar/pull/463))

## [0.0.22](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.21...stellar-scaffold-cli-v0.0.22) - 2026-04-03

### Fixed

- use dest_path for workspace root lookup when updating OZ deps ([#451](https://github.com/theahaco/scaffold-stellar/pull/451))

## [0.0.21](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.20...stellar-scaffold-cli-v0.0.21) - 2026-03-18

### Added

- extension system ([#414](https://github.com/theahaco/scaffold-stellar/pull/414))
- update OZ release to v0.7.0-rc.1 ([#398](https://github.com/theahaco/scaffold-stellar/pull/398))

## [0.0.20](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.19...stellar-scaffold-cli-v0.0.20) - 2026-03-02

### Fixed

- update docker container running alert ([#392](https://github.com/theahaco/scaffold-stellar/pull/392))

### Other

- *(deps)* bump toml_edit from 0.22.27 to 0.23.7 ([#311](https://github.com/theahaco/scaffold-stellar/pull/311))

## [0.0.19](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.18...stellar-scaffold-cli-v0.0.19) - 2026-02-16

### Added

- add managed contracts and `stellar-registry-build` ([#322](https://github.com/theahaco/scaffold-stellar/pull/322))
- update stellar-cli and fix breaking changes ([#324](https://github.com/theahaco/scaffold-stellar/pull/324))

### Fixed

- reorganize RPC tests to simplify filters and fix flaky example tests ([#386](https://github.com/theahaco/scaffold-stellar/pull/386))
- update code for stellar-cli v25, soroban-sdk v25, and admin-sep API changes ([#383](https://github.com/theahaco/scaffold-stellar/pull/383))
- use pedantic clippy and apply suggestions ([#379](https://github.com/theahaco/scaffold-stellar/pull/379))
- use stellar-cli action instead of just setup ([#323](https://github.com/theahaco/scaffold-stellar/pull/323))
- handle npm install errors ([#312](https://github.com/theahaco/scaffold-stellar/pull/312))

### Other

- Chore/accomodate oz v 0.6.0 ([#369](https://github.com/theahaco/scaffold-stellar/pull/369))
- Enable to import contracts from soroban examples ([#327](https://github.com/theahaco/scaffold-stellar/pull/327))
- fetch the most recent supported oz release ([#357](https://github.com/theahaco/scaffold-stellar/pull/357))
- Feat/scaffold clean ([#352](https://github.com/theahaco/scaffold-stellar/pull/352))
- *(deps)* bump cargo_toml from 0.20.5 to 0.22.3 ([#295](https://github.com/theahaco/scaffold-stellar/pull/295))
- Update editions to 2024 across crates ([#292](https://github.com/theahaco/scaffold-stellar/pull/292))
- *(deps)* bump itertools from 0.12.1 to 0.14.0 ([#296](https://github.com/theahaco/scaffold-stellar/pull/296))

## [0.0.18](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.17...stellar-scaffold-cli-v0.0.18) - 2025-11-11

### Fixed

- wrong boolean on git check during init ([#299](https://github.com/theahaco/scaffold-stellar/pull/299))

## [0.0.17](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.16...stellar-scaffold-cli-v0.0.17) - 2025-11-10

### Added

- remove openssl dependency by using rustls-tls feature in reqwest ([#282](https://github.com/theahaco/scaffold-stellar/pull/282))
- pin scaffold FE repo to specific version ([#278](https://github.com/theahaco/scaffold-stellar/pull/278))

### Fixed

- remove unneeded stellar-cli features ([#283](https://github.com/theahaco/scaffold-stellar/pull/283))

### Other

- *(deps)* bump the cargo-minor group across 1 directory with 17 updates ([#284](https://github.com/theahaco/scaffold-stellar/pull/284))
- *(deps)* bump indexmap from 1.9.3 to 2.9.0 ([#234](https://github.com/theahaco/scaffold-stellar/pull/234))
- clean up dependencies ([#272](https://github.com/theahaco/scaffold-stellar/pull/272))
- Update notify package, removes crossbeam warning ([#271](https://github.com/theahaco/scaffold-stellar/pull/271))
- *(deps)* bump predicates from 2.1.5 to 3.1.3 ([#233](https://github.com/theahaco/scaffold-stellar/pull/233))
- *(deps)* bump thiserror from 1.0.69 to 2.0.12 ([#232](https://github.com/theahaco/scaffold-stellar/pull/232))

## [0.0.16](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.15...stellar-scaffold-cli-v0.0.16) - 2025-11-01

### Added

- add install and build steps to init ([#268](https://github.com/theahaco/scaffold-stellar/pull/268))

### Other

- Fix name of second OZ contract ([#262](https://github.com/theahaco/scaffold-stellar/pull/262))

## [0.0.15](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.14...stellar-scaffold-cli-v0.0.15) - 2025-10-29

### Added

- [**breaking**] no longer force users to use local config and add Builder type to remove need for using ENV internall ([#238](https://github.com/theahaco/scaffold-stellar/pull/238))

### Fixed

- *(stellar-scaffold-cli)* use npm.cmd on windows ([#252](https://github.com/theahaco/scaffold-stellar/pull/252))
- update stellar-cli; remove unneeded features for smaller binaries ([#237](https://github.com/theahaco/scaffold-stellar/pull/237))

### Other

- Fix/missing oz contracts ([#261](https://github.com/theahaco/scaffold-stellar/pull/261))
- Try to fix just test-integration ([#260](https://github.com/theahaco/scaffold-stellar/pull/260))

## [0.0.14](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.13...stellar-scaffold-cli-v0.0.14) - 2025-10-20

### Added

- add network in target wasm path ([#213](https://github.com/theahaco/scaffold-stellar/pull/213))

### Other

- recommend to use --locked when installing binary crates ([#235](https://github.com/theahaco/scaffold-stellar/pull/235))
- Copy .env and Run git init  ([#216](https://github.com/theahaco/scaffold-stellar/pull/216))
- *(stellar-registry-cli)* release v0.0.12 ([#208](https://github.com/theahaco/scaffold-stellar/pull/208))

## [0.0.13](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.12...stellar-scaffold-cli-v0.0.13) - 2025-10-03

### Fixed

- default to development environment ([#206](https://github.com/theahaco/scaffold-stellar/pull/206))

## [0.0.12](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.11...stellar-scaffold-cli-v0.0.12) - 2025-09-26

### Fixed

- update all links ([#198](https://github.com/theahaco/scaffold-stellar/pull/198))
- If the packages do not exist, build the clients ([#201](https://github.com/theahaco/scaffold-stellar/pull/201))

## [0.0.11](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.10...stellar-scaffold-cli-v0.0.11) - 2025-09-11

### Fixed

- split dry run into separate action; bump versions to test release ([#185](https://github.com/theahaco/scaffold-stellar/pull/185))

## [0.0.10](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.9...stellar-scaffold-cli-v0.0.10) - 2025-09-11

### Added

- update CD github actions to do a dry run on a release PR ([#183](https://github.com/theahaco/scaffold-stellar/pull/183))
- `watch` command calls upgrade instead of redeploying when possible ([#149](https://github.com/theahaco/scaffold-stellar/pull/149))

## [0.0.9](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.8...stellar-scaffold-cli-v0.0.9) - 2025-09-03

### Fixed

- *(build-clients)* only `allowHttp` in dev & test ([#180](https://github.com/theahaco/scaffold-stellar/pull/180))

## [0.0.8](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.7...stellar-scaffold-cli-v0.0.8) - 2025-08-19

### Other

- Add metadata in upgrade command ([#171](https://github.com/theahaco/scaffold-stellar/pull/171))
- add vite change file test ([#148](https://github.com/theahaco/scaffold-stellar/pull/148))

## [0.0.7](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.6...stellar-scaffold-cli-v0.0.7) - 2025-07-17

### Fixed

- update to newest CLI ([#144](https://github.com/theahaco/scaffold-stellar/pull/144))

### Other

- only watch rust and toml files ([#146](https://github.com/theahaco/scaffold-stellar/pull/146))
- Add version command ([#140](https://github.com/theahaco/scaffold-stellar/pull/140))

## [0.0.6](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.5...stellar-scaffold-cli-v0.0.6) - 2025-07-11

### Fixed

- npm i to output bindings to ensure linkage ([#141](https://github.com/theahaco/scaffold-stellar/pull/141))

## [0.0.5](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.4...stellar-scaffold-cli-v0.0.5) - 2025-07-08

### Added

- use global args and printer in build / watch ([#124](https://github.com/theahaco/scaffold-stellar/pull/124))
- add --no-prompt to upgrade ([#122](https://github.com/theahaco/scaffold-stellar/pull/122))

### Fixed

- init not copying fungible token to proper directory when relative path used ([#127](https://github.com/theahaco/scaffold-stellar/pull/127))
- init flaky test failure ([#130](https://github.com/theahaco/scaffold-stellar/pull/130))
- continue building clients on error ([#88](https://github.com/theahaco/scaffold-stellar/pull/88))

### Other

- Build clients race condition ([#126](https://github.com/theahaco/scaffold-stellar/pull/126))
- add/update NFT token example on init ([#136](https://github.com/theahaco/scaffold-stellar/pull/136))

## [0.0.4](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.3...stellar-scaffold-cli-v0.0.4) - 2025-06-26

### Added

- init retrieves fresh FT interface ([#95](https://github.com/theahaco/scaffold-stellar/pull/95))

### Fixed

- *(registry-cli)* update mainnet instructions to inform about security practices and docs ([#112](https://github.com/theahaco/scaffold-stellar/pull/112))

### Other

- upgrade a project to a scaffold project ([#114](https://github.com/theahaco/scaffold-stellar/pull/114))

## [0.0.3](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.2...stellar-scaffold-cli-v0.0.3) - 2025-06-18

### Added

- add Cargo metadata ([#108](https://github.com/theahaco/scaffold-stellar/pull/108))

### Fixed

- update readmes to point to current directory instead of project ([#109](https://github.com/theahaco/scaffold-stellar/pull/109))
- only copy contents to output location ([#107](https://github.com/theahaco/scaffold-stellar/pull/107))
- ensure account gets funded even if identity already exists ([#106](https://github.com/theahaco/scaffold-stellar/pull/106))

## [0.0.2](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.1...stellar-scaffold-cli-v0.0.2) - 2025-06-11

### Added

- generate contract command ([#84](https://github.com/theahaco/scaffold-stellar/pull/84))

### Fixed

- CD binary builds  ([#97](https://github.com/theahaco/scaffold-stellar/pull/97))
- use correct repo in cargo.toml's
- out_dir fails when building clients ([#91](https://github.com/theahaco/scaffold-stellar/pull/91))

### Other

- Update README.md ([#96](https://github.com/theahaco/scaffold-stellar/pull/96))

## [0.0.1-alpha.2](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.1-alpha.1...stellar-scaffold-cli-v0.0.1-alpha.2) - 2025-05-19

### Added

- Add metadata to wasm and use in registry publish ([#46](https://github.com/theahaco/scaffold-stellar/pull/46))

## [0.0.1-alpha.1](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.1-alpha...stellar-scaffold-cli-v0.0.1-alpha.1) - 2025-05-13

### Other

- docs ([#39](https://github.com/theahaco/scaffold-stellar/pull/39))

