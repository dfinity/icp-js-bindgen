import init, {
  start,
  generate,
  GenerateResult,
} from "./rs/dist/icp-js-bindgen.js";

let initialized = false;

export async function wasmInit() {
  if (initialized) {
    return;
  }

  await init();
  initialized = true;
}

export const wasmStart = start;
export const wasmGenerate = generate;
export type WasmGenerateResult = GenerateResult;
