import { basename, resolve } from 'node:path';
import { prepareBinding, prepareTypescriptBinding } from './bindings.ts';
import { ensureDir, writeFileSafe } from './fs.ts';
import { wasmGenerate, wasmInit } from './rs.ts';

const DID_FILE_EXTENSION = '.did';

/**
 * Options for controlling the generated output files.
 */
export type GenerateOutputOptions = {
  /**
   * If `true`, overwrite existing files. If `false`, abort on collisions.
   *
   * @default false
   */
  force?: boolean;
  /**
   * Options for controlling the generated actor files.
   */
  actor?:
    | {
        /**
         * If `true`, skips generating the actor file (`<service-name>.ts`).
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
  /**
   * Options for controlling the generated declarations files.
   */
  declarations?: {
    /**
     * If `true`, exports root types in the declarations file.
     *
     * @default false
     */
    rootExports?: boolean;
    /**
     * If `true`, generates a single `declarations/<service-name>.did.ts` TypeScript file
     * instead of separate `declarations/<service-name>.did.js` and `declarations/<service-name>.did.d.ts` files.
     *
     * This eliminates the need for `allowJs: true` in the consumer's TypeScript configuration.
     *
     * @default false
     */
    typescript?: boolean;
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
      force: false,
      actor: {
        disabled: false,
        interfaceFile: false,
      },
      declarations: {
        rootExports: false,
      },
    },
  } = options;
  const force = Boolean(output.force); // ensure force is a boolean
  const declarationsRootExports = Boolean(output.declarations?.rootExports ?? false); // ensure rootExports is a boolean
  const declarationsTypescript = Boolean(output.declarations?.typescript ?? false); // ensure typescript is a boolean

  const didFilePath = resolve(didFile);
  const outputFileName = basename(didFile, DID_FILE_EXTENSION);

  await ensureDir(outDir);
  await ensureDir(resolve(outDir, 'declarations'));

  const result = wasmGenerate({
    did_file_path: didFilePath,
    service_name: outputFileName,
    declarations: {
      root_exports: declarationsRootExports,
      typescript: declarationsTypescript,
    },
  });

  // Extract all strings from the WASM object synchronously before any async
  // work and free it explicitly.  The GenerateResult is backed by WASM linear
  // memory and registered with a FinalizationRegistry whose cleanup timing
  // is engine-dependent.  Holding the object across await boundaries has been
  // observed to cause "Out of bounds memory access" traps when multiple
  // generate() calls run concurrently (e.g. several Vite plugin instances).
  const bindings = {
    declarations_js: result.declarations_js,
    declarations_ts: result.declarations_ts,
    declarations_typescript: result.declarations_typescript,
    interface_ts: result.interface_ts,
    service_ts: result.service_ts,
  };
  result.free();

  await writeBindings({
    bindings,
    outDir,
    outputFileName,
    output,
    force,
  });
}

type Bindings = {
  declarations_js: string;
  declarations_ts: string;
  declarations_typescript?: string;
  interface_ts: string;
  service_ts: string;
};

type WriteBindingsOptions = {
  bindings: Bindings;
  outDir: string;
  outputFileName: string;
  output: GenerateOutputOptions;
  force: boolean;
};

async function writeBindings({
  bindings,
  outDir,
  outputFileName,
  output,
  force,
}: WriteBindingsOptions) {
  if (output.declarations?.typescript) {
    const declarationsTypescriptFile = resolve(outDir, 'declarations', `${outputFileName}.did.ts`);
    const declarationsTypescript = prepareTypescriptBinding(
      bindings.declarations_typescript as string,
    );
    await writeFileSafe(declarationsTypescriptFile, declarationsTypescript, force);
  } else {
    const declarationsTsFile = resolve(outDir, 'declarations', `${outputFileName}.did.d.ts`);
    const declarationsJsFile = resolve(outDir, 'declarations', `${outputFileName}.did.js`);

    const declarationsTs = prepareBinding(bindings.declarations_ts);
    const declarationsJs = prepareBinding(bindings.declarations_js);

    await writeFileSafe(declarationsTsFile, declarationsTs, force);
    await writeFileSafe(declarationsJsFile, declarationsJs, force);
  }

  if (output.actor?.disabled) {
    return;
  }

  const serviceTsFile = resolve(outDir, `${outputFileName}.ts`);
  const serviceTs = prepareBinding(bindings.service_ts);
  await writeFileSafe(serviceTsFile, serviceTs, force);

  if (output.actor?.interfaceFile) {
    const interfaceTsFile = resolve(outDir, `${outputFileName}.d.ts`);
    const interfaceTs = prepareBinding(bindings.interface_ts);
    await writeFileSafe(interfaceTsFile, interfaceTs, force);
  }
}
