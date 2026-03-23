---
title: Library
prev: false
---

Use the package as a library to generate bindings for a `.did` file in your own JS/TS scripts (Node.js, Bun, etc.).

> **Note**: The `@icp-sdk/bindgen` library is NOT meant to be used in browsers.

## Installation

```bash
npm install -D @icp-sdk/bindgen
```

## Usage

```ts
import { generate } from '@icp-sdk/bindgen/core';

generate({
  didFile: './canisters/hello_world.did',
  outDir: './src/bindings',
});
```

To write declaration files directly into `outDir` instead of the default `declarations/` subfolder, use the `output.declarations.flat` option:

```ts
generate({
  didFile: './canisters/hello_world.did',
  outDir: './src/bindings',
  output: { declarations: { flat: true } },
});
```

For an explanation of the generated files, see the [Bindings Structure](../structure.md) page.
