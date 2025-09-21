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
