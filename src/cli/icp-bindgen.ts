#!/usr/bin/env node

/**
 * The CLI is used to generate bindings for a `.did` file.
 *
 * ## Installation
 *
 * As a dev dependency for your project:
 *
 * ```bash
 * npm install -D @icp-sdk/bindgen
 * ```
 *
 * Or as a global command:
 *
 * ```bash
 * npm install -g @icp-sdk/bindgen
 * ```
 *
 * ## Usage
 *
 * Suppose you have a `./canisters/hello_world.did` file, and you want to output the generated bindings for your Vite app in the `src/bindings/` folder.
 *
 * You can generate the bindings with the following command:
 *
 * ```bash
 * icp-bindgen --did-file ./canisters/hello_world.did --out-dir ./src/bindings
 * ```
 *
 * For an explanation of the generated files, see the [Bindings Structure](https://js.icp.build/bindgen/latest/structure/) page.
 *
 * ### Usage without installation
 *
 * ```bash
 * npx @icp-sdk/bindgen --did-file ./canisters/hello_world.did --out-dir ./src/bindings
 * ```
 *
 * ### Options
 *
 * - `--did-file <path>`: Path to the `.did` file to generate bindings from
 * - `--out-dir <dir>`: Directory where the bindings will be written
 * - `--actor-interface-file`: If set, generates a `<service-name>.d.ts` file that contains the same types of the `<service-name>.ts` file. Has no effect if `--actor-disabled` is set. (default: `false`)
 * - `--actor-disabled`: If set, skips generating the actor file (`<service-name>.ts`). (default: `false`)
 * - `--force`: If set, overwrite existing files instead of aborting. (default: `false`)
 *
 * @module cli
 */

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

  console.log(cyan(`[${BIN_NAME}] Generating bindings from`), green(didFile));
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
  console.log(cyan(`[${BIN_NAME}] Bindings generated at`), green(outDir));
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
