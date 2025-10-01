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
  interfaceFile?: boolean;
  disableActor?: boolean;
};

async function run(args: Args) {
  const { didFile, outDir, interfaceFile, disableActor } = args;

  console.log(cyan(`[${CLI_NAME}] Generating bindings...`));
  await generate({
    didFile,
    outDir,
    output: {
      interfaceFile,
      disableActor,
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
    '--interface-file',
    'If set, generates a `<service-name>.d.ts` file that contains the same types of the `<service-name>.ts` file. Cannot be used with `--disable-actor`.',
    false,
  )
  .option(
    '--disable-actor',
    'If set, only generates the declarations folder, skipping the actor file (index.ts) and service wrapper file (<service-name>.ts). Cannot be used with `--interface-file`.',
    false,
  )
  .action(run);

program.parseAsync(process.argv).catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
