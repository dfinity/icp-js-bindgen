import path from "node:path";
import { vi } from "vitest";
import { wasmInit } from "../../src/core/generate/rs.ts";

type NodeFsPromises = typeof import("node:fs/promises");

const WASM_PATH = path.resolve(
  import.meta.dirname,
  "../../src/core/generate/rs/dist/icp-js-bindgen_bg.wasm"
);

export async function getWasm(): Promise<{
  wasm: Buffer;
  resolvedFilePath: string;
}> {
  const resolvedUrl = (await import(`${WASM_PATH}?url`)).default;
  const resolvedFilePath = `.${resolvedUrl}`;
  const realFs = await vi.importActual<NodeFsPromises>("node:fs/promises");
  const wasm = await realFs.readFile(resolvedFilePath);
  return { wasm, resolvedFilePath };
}

export async function testWasmInit() {
  const { wasm } = await getWasm();
  await wasmInit({ module_or_path: wasm });
}
