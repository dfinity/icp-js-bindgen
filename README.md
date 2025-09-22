# ICP JS SDK Bindgen

Generate modern TypeScript bindings for Internet Computer (IC) canisters from [Candid](https://github.com/dfinity/candid) `.did` files.

The tool can be used in three ways:
- [CLI](https://js.icp.build/bindgen/latest/cli)
- [Vite plugin](https://js.icp.build/bindgen/latest/plugins/vite)
- [Library](https://js.icp.build/bindgen/latest/core)

> Note: Generated code references `@icp-sdk/core` (agents, candid, principals). Install it in your app if you plan to compile/run the generated code. See `https://js.icp.build/core`.

## Install

```bash
npm i -D @icp-sdk/bindgen
```

## Quick start (Vite)

Generate bindings from a `hello_world.did` file into `src/bindings`:

```ts
// vite.config.ts
import { defineConfig } from 'vite';
import { icpBindgen } from '@icp-sdk/bindgen/plugins/vite';

export default defineConfig({
  plugins: [
    icpBindgen({
      didFile: './hello_world.did',
      outDir: './src/bindings',
    }),
  ],
});
```

Use the generated client:

```ts
import { createActor } from './bindings/hello_world';

const actor = createActor('your-canister-id');
const greeting = await actor.greet('World');
```

For more info, see the [docs](https://js.icp.build/bindgen/).

## Contributing

Contributions are welcome! Please see the [contribution guide](./CONTRIBUTING.md) for more information.

## License

This project is licensed under the [Apache-2.0](./LICENSE) license.
