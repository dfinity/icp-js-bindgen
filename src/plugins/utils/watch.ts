import { resolve } from 'node:path';
import type { ViteDevServer } from 'vite';
import { generate } from '../../core/generate/index.ts';
import type { Options } from '../index.ts';
import { VITE_PLUGIN_NAME } from './constants.ts';
import { cyan, green } from './log.ts';

export function watchDidFileChanges(server: ViteDevServer, options: Options) {
  const didFilePath = resolve(options.didFile);

  server.watcher.add(didFilePath);
  server.watcher.on('change', async (changedPath) => {
    if (resolve(changedPath) === resolve(didFilePath)) {
      console.log(cyan(`[${VITE_PLUGIN_NAME}] Generating bindings...`));

      await generate(options);

      console.log(
        cyan(`[${VITE_PLUGIN_NAME}] Generated bindings successfully at`),
        green(options.outDir),
      );
    }
  });
}
