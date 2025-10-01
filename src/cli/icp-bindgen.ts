#!/usr/bin/env node

import { Command } from 'commander';
import { BIN_NAME, PACKAGE_VERSION } from '../core/constants.ts';
import { generate } from '../core/generate/index.ts';
import { cyan, green, red } from '../plugins/utils/log.ts';

type Args = {
  didFile: string;
  outDir: string;
  actorInterfaceFile?: boolean;
  actorDisabled?: boolean;
  force?: boolean;
};

async function run(args: Args) {
  const { didFile, outDir, actorInterfaceFile, actorDisabled, force } = args;

  console.log(cyan(`[${BIN_NAME}] Generating bindings...`));
  await generate({
    didFile,
    outDir,
    output: {
      force,
      actor: {
        disabled: actorDisabled,
        interfaceFile: actorInterfaceFile,
      },
    },
  });
  console.log(cyan(`[${BIN_NAME}] Generated bindings successfully at`), green(outDir));
}

const program = new Command();

program
  .name(BIN_NAME)
  .version(PACKAGE_VERSION)
  .description('Generate JavaScript bindings for IC canisters')
  .showHelpAfterError()
  .showSuggestionAfterError()
  .requiredOption('--did-file <path>', 'Path to the .did file to generate bindings from')
  .requiredOption('--out-dir <dir>', 'Directory where the bindings will be written')
  .option('--actor-disabled', 'If set, skips generating the actor file (<service-name>.ts).', false)
  .option(
    '--actor-interface-file',
    'If set, generates a `<service-name>.d.ts` file that contains the same types of the `<service-name>.ts` file. Has no effect if `--actor-disabled` is set.',
    false,
  )
  .option('--force', 'If set, overwrite existing files instead of aborting.', false)
  .action(run);

program.parseAsync(process.argv).catch((error) => {
  let errorMessage = '';
  if (error instanceof Error) {
    errorMessage = error.message;
  } else {
    errorMessage = String(error);
  }
  console.error(red(`[${BIN_NAME}] Error: ${errorMessage}`));

  process.exitCode = 1;
});
