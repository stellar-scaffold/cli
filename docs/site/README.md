# Website

This website is built using [Docusaurus](https://docusaurus.io/), a modern static website generator.

## Installation

```bash
npm install
```

## Local Development

```bash
npm run start
```

This command starts a local development server and opens up a browser window. Most changes are reflected live without having to restart the server.

## Build

```bash
npm run build
```

This command generates static content into the `build` directory and can be served using any static contents hosting service.

## Deployment

There is a GitHub action deploying automatically the website when pushing changes to the website folder on main.

```bash
USE_SSH=true npm run deploy
```

We are using GitHub pages for hosting, this command is a convenient way to build the website and push to the `gh-pages` branch.
