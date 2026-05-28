# Contributing to Scaffold Stellar

Thanks for taking the time to improve Scaffold Stellar!

The following is a set of guidelines for contributions and may change over time. Feel free to suggest improvements to this document in a pull request. We want to make it as easy as possible to contribute changes that help this project, and the Stellar ecosystem, grow and thrive. There are a few guidelines that we ask contributors to follow so that we can merge your changes quickly.

## Getting started

- Make sure you have a [GitHub account](https://github.com/signup/free).
- If responding to an existing [GitHub issue](https://github.com/stellar-scaffold/cli/issues), comment on that issue to express your interest and intent to work on this issue.
- If an issue does not exist for the work you'd like to contribute, create a GitHub issue.
  - Clearly describe the issue, including steps to reproduce if it is a bug.
- Fork the repository on GitHub.

## Setting up your development environment

### Install required tools

You need a couple build and test tools to work with the crates in the Scaffold Stellar project.

To install `just`, run the following command:

```bash
cargo install just
```

Make sure cargo-binstall is installed. If not, you can install it with:

```bash
cargo install cargo-binstall
```
https://github.com/cargo-bins/cargo-binstall

Check if you have `cargo-nextest` installed. If not, install it with:

```bash
cargo install cargo-nextest
```
https://crates.io/crates/cargo-nextest

### Setup your dev environment

To set up the environment, run:

```bash
just setup
```

### Running the tests

To run tests, use:

```bash
just test
just test-integration
```

## Changes to the Frontend Template

If you want to make a change to the project scaffold created by the `init` command, you can do so at [the frontend template repo](https://github.com/stellar-scaffold/ui). The `init` command accepts a `--tag` argument if you want to use a specific version of the template other than the latest `main` branch:

```bash
# specify a branch name
stellar scaffold init --tag feat/my-feature my-project-name
# specify a tagged commit
stellar scaffold init --tag v1.2.3 my-project-name
```

## Troubleshooting

- If you need to clean the project (remove the target folder and all compiled artifacts), run: `cargo clean`.

- When you first open the project in an IDE with Rust Analyzer, it may start building dependencies in the background: `Building compile-time-deps...`. During this process, the `target` folder may be temporarily locked. If you run `just test` or other build commands before this finishes, you may see errors like: `Blocking waiting for file lock on build directory`. Solution: wait for Rust Analyzer’s background build to complete before running commands.

- If you run `just test` or other commands in WSL (Windows Subsystem for Linux), the build may consume a lot of memory. On machines with limited WSL resources, builds can terminate unexpectedly due to out-of-memory errors. Solution: increase WSL resources by editing (or creating) `C:\Users\YOUR_USER\.wslconfig` file if possible.

- For Windows users, please refer [here](./WINDOWS.md).
