import type {
  GenerateDeclarationsOptions,
  GenerateOptions,
  GenerateResult,
} from './rs/dist/icp-js-bindgen.d.ts';
import init, { generate, start } from './rs/dist/icp-js-bindgen.js';

let initialized = false;

export async function wasmInit(...args: Parameters<typeof init>) {
  if (initialized) {
    return;
  }

  await init(...args);
  initialized = true;
}

export const wasmStart = start;
export const wasmGenerate = generate;
export type WasmGenerateDeclarationsOptions = GenerateDeclarationsOptions;
export type WasmGenerateOptions = GenerateOptions;
export type WasmGenerateResult = GenerateResult;
