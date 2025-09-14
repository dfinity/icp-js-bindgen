import { type Plugin } from 'vite';
import { generate } from '../core/generate/index.ts';
import type { Options } from './index.ts';
import { watchDidFileChanges } from './utils/watch.ts';
import { cyan, green } from './utils/log.ts';
import { VITE_PLUGIN_NAME } from './utils/constants.ts';

export function icpBindgen(options: Options): Plugin {
  return {
    name: VITE_PLUGIN_NAME,
    async buildStart() {
      console.log(cyan(`[${VITE_PLUGIN_NAME}] Generating bindings...`));

      await generate(options);

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
