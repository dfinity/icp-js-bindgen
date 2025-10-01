#!/usr/bin/env node

import { Command } from 'commander';
import { generate } from '../core/generate/index.ts';
import { cyan, green } from '../plugins/utils/log.ts';

/**
 * Defined in package.json and injected by Vite. See vite.config.ts.
 */
declare const __PKG_VERSION__: string;

const CLI_NAME = 'icp-bindgen';

type Args = {
  didFile: string;
  outDir: string;
  actorInterfaceFile?: boolean;
  actorDisabled?: boolean;
};

async function run(args: Args) {
  const { didFile, outDir, actorInterfaceFile, actorDisabled } = args;

  console.log(cyan(`[${CLI_NAME}] Generating bindings...`));
  await generate({
    didFile,
    outDir,
    output: {
      actor: {
        disabled: actorDisabled,
        interfaceFile: actorInterfaceFile,
      },
    },
  });
  console.log(cyan(`[${CLI_NAME}] Generated bindings successfully at`), green(outDir));
}

const program = new Command();

program
  .name(CLI_NAME)
  .version(__PKG_VERSION__)
  .description('Generate JavaScript bindings for IC canisters')
  .showHelpAfterError()
  .showSuggestionAfterError()
  .requiredOption('--did-file <path>', 'Path to the .did file to generate bindings from')
  .requiredOption('--out-dir <dir>', 'Directory where the bindings will be written')
  .option(
    '--actor-disabled',
    'If set, skips generating the actor file (<service-name>.ts).',
    false,
  )
  .option(
    '--actor-interface-file',
    'If set, generates a `<service-name>.d.ts` file that contains the same types of the `<service-name>.ts` file. Has no effect if `--actor-disabled` is set.',
    false,
  )
  .action(run);

program.parseAsync(process.argv).catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
