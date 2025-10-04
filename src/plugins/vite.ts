/**
 * The Vite plugin is used to generate bindings for a `.did` file during the build process.
 *
 * ## Installation
 *
 * ```bash
 * npm install -D @icp-sdk/bindgen
 * ```
 *
 * ## Usage
 *
 * Suppose you have a `./canisters/hello_world.did` file, and you want to output the generated bindings for your Vite app in the `src/bindings/` folder.
 * Here's how the plugin configuration would look like:
 *
 * ```ts title="vite.config.ts"
 * import { defineConfig } from "vite";
 * import { icpBindgen } from '@icp-sdk/bindgen/plugins/vite';
 *
 * export default defineConfig({
 *   plugins: [
 *     // ... other plugins
 *     icpBindgen({
 *       didFile: './canisters/hello_world.did',
 *       outDir: './src/bindings',
 *     }),
 *   ],
 * });
 * ```
 *
 * For an explanation of the generated files, see the [Bindings Structure](https://js.icp.build/bindgen/latest/structure/) page.
 *
 * @module plugins/vite
 */

import { resolve } from 'node:path';
import type { Plugin, ViteDevServer } from 'vite';
import {
  type GenerateOptions,
  type GenerateOutputOptions,
  generate,
} from '../core/generate/index.ts';
import { VITE_PLUGIN_NAME } from './utils/constants.ts';
import { cyan, green } from './utils/log.ts';

/**
 * Options for the Vite plugin.
 */
export interface Options extends Omit<GenerateOptions, 'output'> {
  /**
   * Options for controlling the generated output files.
   */
  output?: Omit<GenerateOutputOptions, 'force'>;
  /**
   * Disables watching for changes in the `.did` file when using the dev server.
   *
   * @default false
   */
  disableWatch?: boolean;
}

/**
 * Vite plugin to generate bindings for a `.did` file during the build process.
 *
 * The plugin also watches the `.did` file in dev mode and regenerates the bindings on change.
 * You can disable this behavior by setting `disableWatch` to `true`.
 *
 * For more info, see the [docs](https://js.icp.build/bindgen/latest/plugins/vite).
 *
 * @example
 *
 * Suppose we have a `.did` file in `./canisters/hello_world.did` and we want to generate bindings in `./src/bindings`.
 *
 * ```ts
 * // vite.config.ts
 * import { defineConfig } from 'vite';
 * import { icpBindgen } from '@icp-sdk/bindgen/plugins/vite';
 *
 * export default defineConfig({
 *   plugins: [
 *     // ... other plugins
 *     icpBindgen({
 *       didFile: './canisters/hello_world.did',
 *       outDir: './src/bindings',
 *     }),
 *   ],
 * });
 * ```
 *
 * @ignore
 */
export function icpBindgen(options: Options): Plugin {
  return {
    name: VITE_PLUGIN_NAME,
    async buildStart() {
      await run(options);
    },
    configureServer(server) {
      if (!options.disableWatch) {
        watchDidFileChanges(server, options);
      }
    },
    sharedDuringBuild: true,
  };
}

function watchDidFileChanges(server: ViteDevServer, options: Options) {
  const didFilePath = resolve(options.didFile);

  server.watcher.add(didFilePath);
  server.watcher.on('change', async (changedPath) => {
    if (resolve(changedPath) === resolve(didFilePath)) {
      await run(options);
    }
  });
}

async function run(options: Options) {
  console.log(cyan(`[${VITE_PLUGIN_NAME}] Generating bindings from`), green(options.didFile));

  await generate({
    didFile: options.didFile,
    outDir: options.outDir,
    output: {
      ...options.output,
      // We want to overwrite existing files in the build process
      force: true,
    },
    additionalFeatures: options.additionalFeatures,
  });

  console.log(cyan(`[${VITE_PLUGIN_NAME}] Bindings generated at`), green(options.outDir));
}
