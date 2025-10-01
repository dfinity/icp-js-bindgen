---
title: Bindings Structure
prev: false
next: false
---

The tool writes the files in the specified output directory with the following structure. Assuming you have a `<service-name>.did` file, `<service-name>` is used for the generated files.

> Note: Generated code references `@icp-sdk/core` (agents, candid, principals). Install it in your app if you plan to compile/run the generated code. See `https://js.icp.build/core`.

## `declarations/`

This folder contains the actual Candid JS bindings. It generates the same bindings that the [`dfx generate`](https://internetcomputer.org/docs/building-apps/developer-tools/dfx/dfx-generate) command was generating.

### `declarations/<service-name>.did.d.ts`

This file is used in TypeScript projects to type the Candid JS bindings generated in [`declarations/<service-name>.did.js`](#declarationsservice-namedidjs).

### `declarations/<service-name>.did.js`

This file contains the actual Candid JS bindings, that allow encoding and decoding JS objects to and from Candid.

## `<service-name>.ts`

This file contains the TypeScript wrapper for the Candid JS bindings generated in [`declarations/<service-name>.did.js`](#declarationsservice-namedidjs). It offers a more idiomatic and type-safe TypeScript interface over the Candid JS bindings. Set the [`output.declarationsOnly`](./core/api/type-aliases/GenerateOutputOptions.md#declarationsOnly) option to `true` to skip generating this file.

## `index.ts`

This file contains the `createActor` function that can be used to instantiate the Candid JS bindings generated in [`declarations/<service-name>.did.js`](#declarationsservice-namedidjs). Set the [`output.declarationsOnly`](./core/api/type-aliases/GenerateOutputOptions.md#declarationsOnly) option to `true` to skip generating this file.

Here's an example of how to use the generated client:

```ts
import { createActor } from './bindings/hello_world';

const actor = createActor('your-canister-id');
const greeting = await actor.greet('World');
```

## Optional files

### `<service-name>.d.ts`

This file contains the same TypeScript types as [`<service-name>.ts`](#service-namets). It is typically used to add to LLMs' contexts' to give knowledge about what types are available in the service. Set the [`output.interfaceFile`](./core/api/type-aliases/GenerateOutputOptions.md#interfaceFile) option to `true` to generate this file.

### `canister-env.d.ts`

This file contains the strongly-typed canister environment variables. It is typically used make the `@icp-sdk/canister-env` package more type-safe. configure the [`additionalFeatures.canisterEnv`](./core/api/type-aliases/GenerateAdditionalFeaturesOptions.md#canisterEnv) option to generate this file.

> Not supported by the CLI yet.
