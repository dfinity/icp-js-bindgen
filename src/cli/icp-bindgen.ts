#!/usr/bin/env node

import { Command } from "commander";
import { generate } from "../core/generate/index.ts";
import { cyan, green } from "../plugins/utils/log.ts";

// defined in vite.config.ts
declare const __PKG_VERSION__: string;

const CLI_NAME = "icp-bindgen";

type Args = {
  didFile: string;
  outDir: string;
};

async function run(args: Args) {
  const { didFile, outDir } = args;

  console.log(cyan(`[${CLI_NAME}] Generating bindings...`));
  await generate({ didFile, outDir });
  console.log(
    cyan(`[${CLI_NAME}] Generated bindings successfully at`),
    green(outDir)
  );
}

const program = new Command();

program
  .name(CLI_NAME)
  .version(__PKG_VERSION__)
  .description("Generate JavaScript bindings for IC canisters")
  .showHelpAfterError()
  .showSuggestionAfterError()
  .requiredOption(
    "--did-file <path>",
    "Path to the .did file to generate bindings from"
  )
  .requiredOption(
    "--out-dir <dir>",
    "Directory where the bindings will be written"
  )
  .action(async (options: Args) => {
    await run(options);
  });

program.parseAsync(process.argv).catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
