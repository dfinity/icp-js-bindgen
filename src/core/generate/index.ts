import { writeFile } from 'node:fs/promises';
import { basename, resolve } from 'node:path';
import { indexBinding, prepareBinding } from './bindings.ts';
import {
  type GenerateAdditionalFeaturesOptions,
  generateAdditionalFeatures,
} from './features/index.ts';
import { emptyDirWithFilter, ensureDir } from './fs.ts';
import { type WasmGenerateResult, wasmGenerate, wasmInit } from './rs.ts';

export type { GenerateAdditionalFeaturesOptions } from './features/index.ts';

const DID_FILE_EXTENSION = '.did';

/**
 * Options for controlling the generated output files.
 */
export type GenerateOutputOptions = {
  /**
   * Options for controlling the generated `index.ts` and `<service-name>.ts` files.
   */
  actor?:
    | {
        /**
         * If `true`, skips generating the actor file (`index.ts`) and service wrapper file (`<service-name>.ts`).
         *
         * @default false
         */
        disabled: true;
      }
    | {
        disabled?: false;
        /**
         * If `true`, generates a `<service-name>.d.ts` file that contains the same types of the `<service-name>.ts` file.
         * Useful to add to LLMs' contexts' to give knowledge about what types are available in the service.
         *
         * Has no effect if `disabled` is `true`.
         *
         * @default false
         */
        interfaceFile?: boolean;
      };
};

/**
 * Options for the {@link generate} function.
 */
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
   * Options for controlling the generated output files.
   */
  output?: GenerateOutputOptions;
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

  const {
    didFile,
    outDir,
    output = {
      actor: {
        disabled: false,
        interfaceFile: false,
      },
    },
  } = options;

  const didFilePath = resolve(didFile);
  const outputFileName = basename(didFile, DID_FILE_EXTENSION);

  await ensureDir(outDir);
  await emptyDirWithFilter(outDir, (path) => !path.endsWith(DID_FILE_EXTENSION));
  await ensureDir(resolve(outDir, 'declarations'));

  const result = wasmGenerate(didFilePath, outputFileName);

  await writeBindings({
    bindings: result,
    outDir,
    outputFileName,
    output,
  });

  if (options.additionalFeatures) {
    await generateAdditionalFeatures(options.additionalFeatures, options.outDir);
  }
}

type WriteBindingsOptions = {
  bindings: WasmGenerateResult;
  outDir: string;
  outputFileName: string;
  output: GenerateOutputOptions;
};

async function writeBindings({ bindings, outDir, outputFileName, output }: WriteBindingsOptions) {
  const declarationsTsFile = resolve(outDir, 'declarations', `${outputFileName}.did.d.ts`);
  const declarationsJsFile = resolve(outDir, 'declarations', `${outputFileName}.did.js`);

  const declarationsTs = prepareBinding(bindings.declarations_ts);
  const declarationsJs = prepareBinding(bindings.declarations_js);

  await writeFile(declarationsTsFile, declarationsTs);
  await writeFile(declarationsJsFile, declarationsJs);

  if (output.actor?.disabled) {
    return;
  }

  const serviceTsFile = resolve(outDir, `${outputFileName}.ts`);
  const serviceTs = prepareBinding(bindings.service_ts);
  await writeFile(serviceTsFile, serviceTs);
  await writeIndex(outDir, outputFileName);

  if (output.actor?.interfaceFile) {
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
