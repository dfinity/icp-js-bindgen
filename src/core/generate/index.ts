import { writeFile } from 'node:fs/promises';
import { basename, resolve } from 'node:path';
import { indexBinding, prepareBinding } from './bindings.ts';
import {
  type GenerateAdditionalFeaturesOptions,
  generateAdditionalFeatures,
} from './features/index.ts';
import { emptyDir, ensureDir } from './fs.ts';
import { type WasmGenerateResult, wasmGenerate, wasmInit } from './rs.ts';

export type { GenerateAdditionalFeaturesOptions } from './features/index.ts';

const DID_FILE_EXTENSION = '.did';

export type GenerateOptions = {
  didFile: string;
  outDir: string;
  additionalFeatures?: GenerateAdditionalFeaturesOptions;
};

/**
 * Generates the bindings for a `.did` file.
 *
 * For an explanation of the generated files, see the [Bindings Structure](https://js.icp.build/bindgen/latest/structure) docs page.
 *
 * @param options - The options for the generate function.
 *
 * @example
 *
 * Suppose we have a `.did` file in `./canisters/hello_world.did` and we want to generate bindings in `./src/bindings`.
 *
 * ```ts
 * await generate({
 *   didFile: './canisters/hello_world.did',
 *   outDir: './src/bindings',
 * });
 * ```
 */
export async function generate(options: GenerateOptions) {
  await wasmInit();

  const { didFile, outDir } = options;
  const didFilePath = resolve(didFile);
  const outputFileName = basename(didFile, DID_FILE_EXTENSION);

  await emptyDir(outDir);
  await ensureDir(outDir);
  await ensureDir(resolve(outDir, 'declarations'));

  const result = wasmGenerate(didFilePath, outputFileName);

  await writeBindings({
    bindings: result,
    outDir,
    outputFileName,
  });

  if (options.additionalFeatures) {
    await generateAdditionalFeatures(options.additionalFeatures, options.outDir);
  }

  await writeIndex(outDir, outputFileName);
}

type WriteBindingsOptions = {
  bindings: WasmGenerateResult;
  outDir: string;
  outputFileName: string;
};

async function writeBindings({ bindings, outDir, outputFileName }: WriteBindingsOptions) {
  const declarationsTsFile = resolve(outDir, 'declarations', `${outputFileName}.did.d.ts`);
  const declarationsJsFile = resolve(outDir, 'declarations', `${outputFileName}.did.js`);
  const interfaceTsFile = resolve(outDir, `${outputFileName}.d.ts`);
  const serviceTsFile = resolve(outDir, `${outputFileName}.ts`);

  const declarationsTs = prepareBinding(bindings.declarations_ts);
  const declarationsJs = prepareBinding(bindings.declarations_js);
  const interfaceTs = prepareBinding(bindings.interface_ts);
  const serviceTs = prepareBinding(bindings.service_ts);

  await writeFile(declarationsTsFile, declarationsTs);
  await writeFile(declarationsJsFile, declarationsJs);
  await writeFile(interfaceTsFile, interfaceTs);
  await writeFile(serviceTsFile, serviceTs);
}

async function writeIndex(outDir: string, outputFileName: string) {
  const indexFile = resolve(outDir, 'index.ts');

  const index = indexBinding(outputFileName);
  await writeFile(indexFile, index);
}
