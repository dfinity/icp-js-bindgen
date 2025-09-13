import path from "node:path";
import { readFile } from "node:fs/promises";
import { wasmInit } from "../../src/core/generate/rs.ts";

const WASM_PATH = path.resolve(
  import.meta.dirname,
  "../../src/core/generate/rs/dist/icp-js-bindgen_bg.wasm"
);

export async function testWasmInit() {
  const resolvedUrl = (await import(`${WASM_PATH}?url`)).default;
  const buffer = await readFile(`.${resolvedUrl}`);
  await wasmInit({ module_or_path: buffer });
}
