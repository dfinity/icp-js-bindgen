---
title: Library
prev: false
---

Use the package as a library to generate bindings for a `.did` file in your own JS/TS scripts (Node.js, Bun, etc.).

> **Note**: This library is meant to be used in browsers.

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

For an explanation of the generated files, see the [Bindings Structure](../structure.md) page.
