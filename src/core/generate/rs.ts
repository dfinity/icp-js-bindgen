import { readFile } from 'node:fs/promises';
import type {
  GenerateDeclarationsOptions,
  GenerateOptions,
  GenerateResult,
} from './rs/dist/icp-js-bindgen.d.ts';
import init, { generate, start } from './rs/dist/icp-js-bindgen.js';

let initPromise: Promise<void> | undefined;

export async function wasmInit(...args: Parameters<typeof init>) {
  if (!initPromise) {
    // If caller didn't pass explicit args, try to load the .wasm file bytes
    // directly when it's a file: URL. This prevents hanging when runtimes
    // don't support fetch(file:) semantics.
    let initArgs = args;

    if (initArgs.length === 0) {
      try {
        const wasmUrl = new URL('./rs/dist/icp-js-bindgen_bg.wasm', import.meta.url);
        const wasmPath = wasmUrl.pathname;
        const bytes = await readFile(wasmPath);
        initArgs = [{ module_or_path: bytes }];
      } catch {
        // If it failes, ignore and fall back to default init behavior
        initArgs = args;
      }
    }

    initPromise = init(...(initArgs)).then(
      () => {},
      (error: unknown) => {
        initPromise = undefined;
        throw error;
      },
    );
  }
  return initPromise;
}

export const wasmStart = start;
export const wasmGenerate = generate;
export type WasmGenerateDeclarationsOptions = GenerateDeclarationsOptions;
export type WasmGenerateOptions = GenerateOptions;
export type WasmGenerateResult = GenerateResult;
