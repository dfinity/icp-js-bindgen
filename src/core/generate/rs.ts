import type {
  GenerateDeclarationsOptions,
  GenerateOptions,
  GenerateResult,
} from './rs/dist/icp-js-bindgen.d.ts';
import init, { generate, start } from './rs/dist/icp-js-bindgen.js';

let initPromise: Promise<void> | undefined;

export async function wasmInit(...args: Parameters<typeof init>) {
  if (!initPromise) {
    initPromise = init(...args).then(
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
