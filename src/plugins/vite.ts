import type { Plugin } from 'vite';
import { type GenerateOptions, generate } from '../core/generate/index.ts';
import { VITE_PLUGIN_NAME } from './utils/constants.ts';
import { cyan, green } from './utils/log.ts';
import { watchDidFileChanges } from './utils/watch.ts';

/**
 * Options for the Vite plugin.
 */
export interface Options extends GenerateOptions {
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
 */
export function icpBindgen(options: Options): Plugin {
  return {
    name: VITE_PLUGIN_NAME,
    async buildStart() {
      console.log(cyan(`[${VITE_PLUGIN_NAME}] Generating bindings...`));

      await generate({
        didFile: options.didFile,
        outDir: options.outDir,
        output: options.output,
        additionalFeatures: options.additionalFeatures,
      });

      console.log(
        cyan(`[${VITE_PLUGIN_NAME}] Generated bindings successfully at`),
        green(options.outDir),
      );
    },
    configureServer(server) {
      if (!options.disableWatch) {
        watchDidFileChanges(server, options);
      }
    },
    sharedDuringBuild: true,
  };
}
