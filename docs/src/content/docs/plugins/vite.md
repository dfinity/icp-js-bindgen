---
title: Vite Plugin
next: false
prev: false
---

The Vite plugin is used to generate bindings for a `.did` file during the build process.

## Installation

```bash
npm install -D @icp-sdk/bindgen
```

## Usage

Suppose you have a `./canisters/hello_world.did` file, and you want to output the generated bindings for your Vite app in the `src/bindings/` folder.
Here's how the plugin configuration would look like:

```ts title="vite.config.ts"
import { defineConfig } from "vite";
import { icpBindgen } from '@icp-sdk/bindgen/plugins/vite';

export default defineConfig({
  plugins: [
    // ... other plugins
    icpBindgen({
      didFile: './canisters/hello_world.did',
      outDir: './src/bindings',
    }),
  ],
});
```

For an explanation of the generated files, see the [Bindings Structure](../structure.md) page.

### Options

#### `didFile`

The path to the `.did` file to generate bindings from.

#### `outDir`

The directory where the bindings will be written.

#### `output`

Options for controlling the generated output files.

##### `disableActor`

If `true`, only generates the declarations folder and skips generating the actor file (`index.ts`) and service wrapper file (`<service-name>.ts`).

> **Note**: If `true`, [`interfaceFile`](#interfaceFile) must be `false`.

##### `interfaceFile`

If `true`, generates a `<service-name>.d.ts` file that contains the same types of the `<service-name>.ts` file.
Useful to add to LLMs' contexts' to give knowledge about what types are available in the service.

> **Note**: If `true`, [`disableActor`](#disableActor) must be `false`.

#### `additionalFeatures`

Additional features to generate bindings with.

##### `canisterEnv`

If defined, generates a `canister-env.d.ts` file according to the provided options. Just configure the `variableNames` property with the canister environment variables that you'd like to use in the frontend app.

#### `disableWatch`

Disables watching for changes in the `.did` file when using the dev server.
