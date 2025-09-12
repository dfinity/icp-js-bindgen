import * as bindgen from "./rs/dist/icp-js-bindgen";

export const wasmStart = bindgen.start;
export const wasmGenerate = bindgen.generate;
export type WasmGenerateResult = bindgen.GenerateResult;
