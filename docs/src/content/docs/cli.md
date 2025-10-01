---
title: CLI
---

The CLI is used to generate bindings for a `.did` file.

## Installation

As a dev dependency for your project:

```bash
npm install -D @icp-sdk/bindgen
```

Or as a global command:

```bash
npm install -g @icp-sdk/bindgen
```

## Usage

Suppose you have a `./canisters/hello_world.did` file, and you want to output the generated bindings for your Vite app in the `src/bindings/` folder.

You can generate the bindings with the following command:

```bash
icp-bindgen --did-file ./canisters/hello_world.did --out-dir ./src/bindings
```

For an explanation of the generated files, see the [Bindings Structure](./structure.md) page.

### Usage without installation

```bash
npx @icp-sdk/bindgen --did-file ./canisters/hello_world.did --out-dir ./src/bindings
```

### Options

- `--did-file <path>`: Path to the `.did` file to generate bindings from
- `--out-dir <dir>`: Directory where the bindings will be written
- `--actor-interface-file`: If set, generates a `<service-name>.d.ts` file that contains the same types of the `<service-name>.ts` file. Has no effect if `--actor-disabled` is set.
- `--actor-disabled`: If set, skips generating the actor file (`<service-name>.ts`)
- `--force`: If set, overwrite existing files instead of aborting.

> **Note**: The CLI does not support additional features yet.
