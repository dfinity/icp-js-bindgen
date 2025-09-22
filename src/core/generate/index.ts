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
  /**
   * The path to the `.did` file.
   */
  didFile: string;
  /**
   * The path to the directory where the bindings will be generated.
   */
  outDir: string;
  /**
   * If `true`, generates a `<service-name>.d.ts` file that contains the same types of the `<service-name>.ts` file.
   * Useful to add to LLMs' contexts' to give knowledge about what types are available in the service.
   *
   * @default false
   */
  interfaceDeclaration?: boolean;
  /**
   * Additional features to generate bindings with.
   */
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

  const { didFile, outDir, interfaceDeclaration = false } = options;
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
    interfaceDeclaration,
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
  interfaceDeclaration: boolean;
};

async function writeBindings({
  bindings,
  outDir,
  outputFileName,
  interfaceDeclaration,
}: WriteBindingsOptions) {
  const declarationsTsFile = resolve(outDir, 'declarations', `${outputFileName}.did.d.ts`);
  const declarationsJsFile = resolve(outDir, 'declarations', `${outputFileName}.did.js`);
  const serviceTsFile = resolve(outDir, `${outputFileName}.ts`);

  const declarationsTs = prepareBinding(bindings.declarations_ts);
  const declarationsJs = prepareBinding(bindings.declarations_js);
  const serviceTs = prepareBinding(bindings.service_ts);

  await writeFile(declarationsTsFile, declarationsTs);
  await writeFile(declarationsJsFile, declarationsJs);
  await writeFile(serviceTsFile, serviceTs);

  if (interfaceDeclaration) {
    const interfaceTsFile = resolve(outDir, `${outputFileName}.d.ts`);
    const interfaceTs = prepareBinding(bindings.interface_ts);
    await writeFile(interfaceTsFile, interfaceTs);
  }
}

async function writeIndex(outDir: string, outputFileName: string) {
  const indexFile = resolve(outDir, 'index.ts');

  const index = indexBinding(outputFileName);
  await writeFile(indexFile, index);
}
