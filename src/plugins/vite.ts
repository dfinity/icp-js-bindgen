import type { Plugin } from 'vite';
import { generate } from '../core/generate/index.ts';
import type { Options } from './index.ts';
import { VITE_PLUGIN_NAME } from './utils/constants.ts';
import { cyan, green } from './utils/log.ts';
import { watchDidFileChanges } from './utils/watch.ts';

/**
 * Vite plugin to generate bindings for a `.did` file during the build process.
 *
 * The plugin also watches the `.did` file in dev mode and regenerates the bindings on change.
 * You can disable this behavior by setting `disableWatch` to `true`.
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
 * For more info, see the [docs](https://js.icp.build/bindgen/latest/plugins/vite).
 */
export function icpBindgen(options: Options): Plugin {
  return {
    name: VITE_PLUGIN_NAME,
    async buildStart() {
      console.log(cyan(`[${VITE_PLUGIN_NAME}] Generating bindings...`));

      await generate({
        didFile: options.didFile,
        outDir: options.outDir,
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

export type { Options };
