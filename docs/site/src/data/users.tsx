/**
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

/* eslint-disable global-require */

import {translate} from '@docusaurus/Translate';
import {sortBy} from '@site/src/utils/jsUtils';

/*
 * ADD YOUR SITE TO THE SCAFFOLD STELLAR SHOWCASE
 *
 * Please submit a PR yourself
 *
 * Instructions:
 * - Add the site in the json array below
 * - `title` is the project's name (no need for the "Docs" suffix)
 * - A short (≤120 characters) description of the project
 * - Use relevant tags to categorize the site (some further clarifications below)
 * - Add a local image preview (decent screenshot of the site)
 * - The image MUST be added to the GitHub repository, and use `require("img")`
 * - The image has to have minimum width 640 and an aspect of no wider than 2:1
 * - Resize images: node resizeImage.js
 * - Reduce size by running pngcrush or pngquant. E.g. `pngquant --ext .png --force image.png`
 * - Open a PR and check for reported CI errors
 *
 */

// LIST OF AVAILABLE TAGS
// Available tags to assign to a showcase site
// Please choose all tags that you think might apply.
// We'll remove inappropriate tags, but it's less likely that we add tags.
export type TagType =
  // DO NOT USE THIS TAG: we choose sites to add to favorites
  | 'favorite'
  // great smart contract
  | 'contract'
  | 'frontend'
  // winner of a hackathon
  | 'hackathon'

// Add sites to this list
// prettier-ignore
const Users: User[] = [
  {
    title: 'SAMPLED',
    description: 'The Decentralized Sample Marketplace Where Producers Get Paid Instantly on Stellar',
    preview: require('./showcase/sampled.png'),
    website: 'https://www.stellarsampled.com/',
    source: 'https://github.com/osas2211/sampled/',
    tags: ['frontend', 'hackathon'],
  },
  {
    title: 'SecureFlow',
    description: 'Create a secure, transparent, and fair platform for freelancers and clients to collaborate with built-in payment protection and dispute resolution.',
    preview: require('./showcase/secureflow.png'),
    website: 'https://secure-flow-scaffold.vercel.app/',
    source: 'https://github.com/Gbangbolaoluwagbemiga/SecureFlow-scaffold',
    tags: ['hackathon'],
  },
  {
    title: 'Splicers',
    description: 'The surface world is lost. In subterranean bunkers, survivors fuse genes and print monsters — the ultimate fighters, born to reclaim the world that once was.',
    preview: require('./showcase/splicers.png'),
    website: 'https://splicers.net',
    source: 'https://github.com/AshFrancis/splicers',
    tags: ['favorite', 'contract', 'hackathon'],
  },
  {
    title: 'Sudoku Proof Generator',
    description: 'Verify Noir Ultrahonk Zero Knowledge proof with Stellar smart contract.',
    preview: require('./showcase/noir-stellar.png'),
    website: 'https://noir-stellar.netlify.app',
    source: 'https://github.com/tupui/ultrahonk_soroban_contract',
    tags: ['favorite', 'contract'],
  },

  /*
  Pro Tip: add your site in alphabetical order.
  Appending your site here (at the end) is more likely to produce Git conflicts.
   */
];

export type User = {
  title: string;
  description: string;
  preview: string | null; // null = use our serverless screenshot service
  website: string;
  source: string | null;
  tags: TagType[];
};

export type Tag = {
  label: string;
  description: string;
  color: string;
};

export const Tags: {[type in TagType]: Tag} = {
  favorite: {
    label: translate({message: 'Favorite'}),
    description: translate({
      message:
        'Our favorite Scaffold Stellar dApp that you must absolutely check out!',
      id: 'showcase.tag.favorite.description',
    }),
    color: '#e9669e',
  },

  contract: {
    label: translate({message: 'Stellar Smart Contract'}),
    description: translate({
      message: 'Great examples of Stellar smart contract!',
      id: 'showcase.tag.contract.description',
    }),
    color: '#39ca30',
  },

  frontend: {
    label: translate({message: 'Frontend'}),
    description: translate({
      message: 'Beautiful Scaffold Stellar dApp, polished and standing out from the initial template!',
      id: 'showcase.tag.frontend.description',
    }),
    color: '#a44fb7',
  },

  hackathon: {
    label: translate({message: 'Hackathon Winner'}),
    description: translate({
      message: 'Scaffold Stellar dApp which won a hackathon!',
      id: 'showcase.tag.hackathon.description',
    }),
    color: '#dfd545',
  },

};

export const TagList = Object.keys(Tags) as TagType[];
function sortUsers() {
  let result = Users;
  // Sort by site name
  result = sortBy(result, (user) => user.title.toLowerCase());
  // Sort by favorite tag, favorites first
  result = sortBy(result, (user) => !user.tags.includes('favorite'));
  return result;
}

export const sortedUsers = sortUsers();
