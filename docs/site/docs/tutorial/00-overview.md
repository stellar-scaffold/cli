---
sidebar_label: Overview
---

# Tutorial Overview

This tutorial will help you learn how to use [Scaffold Stellar](https://github.com/stellar-scaffold/cli) to build and manage smart contracts on the Stellar blockchain and a decentralized application (dApp) to interact with them. Scaffold Stellar is a developer toolkit that provides CLI tools, contract templates, and a starter React UI to get your idea out of your head and on to the network as fast as possible.

:::tip

If you just want to get up and running quickly, check out the [Quick Start](../quick-start.mdx) guide.

:::

## 🎯 What will we build?

Our smart contract will be a Guess The Number game. You (the admin) can deploy the contract, randomly select a number between 1 and 10, and seed the contract with a prize. Users can make guesses and win the prize if they're correct!

We'll use Scaffold Stellar to create the initial project structure containing contract code and a frontend application to interact with it. It will handle all the heavy lifting for us, letting us focus on the game logic in the contract and immediately build up the frontend for users to play the game.

## 📋Prerequisites

Before jumping in, you should have a basic understanding of the command line and of general programming concepts, but we'll walk through all the code together so don't worry if you're new to Stellar, Rust, or dApp development. We'll link out to [The Rust Programming Language book](https://doc.rust-lang.org/stable/book/) to explain concepts if you want to dive deeper.

## 📑 Contents

This tutorial is split into four sections:

1. [Getting Started](./01-getting-started.md): will help you setup your development environment, initialize a new project, and explain the architecture and contract code
2. [Making Improvements](./02-making-improvements.md): will explain the front-end architecture and more CLI tooling to get you used to the development workflow
3. [Adding Transactions](./03-adding-payments.md): will show examples of working with real transactions of XLM in smart contracts and interacting with wallets in the dApp
4. [Best Practices](./04-best-practices.md): will add some final polish to our contract to make sure everything is ready to be deployed to production

Well, what are you waiting for? [Get started!](./01-getting-started.md)
