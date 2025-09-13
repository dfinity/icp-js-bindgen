import { describe, it, expect, beforeAll } from "vitest";
import { wasmGenerate } from "../src/core/generate/rs.ts";
import { testWasmInit } from "./utils/wasm.ts";

const TESTS_ASSETS_DIR = "./tests/assets";
const SNAPSHOTS_DIR = "./assets/snapshots";

beforeAll(async () => {
  await testWasmInit();
});

describe("wasmGenerate", () => {
  it("should generate a bindgen", async () => {
    const helloWorldServiceName = "hello_world";
    const helloWorldDidFile = `${TESTS_ASSETS_DIR}/${helloWorldServiceName}.did`;

    const result = wasmGenerate(helloWorldDidFile, helloWorldServiceName);
    await expect(result.declarations_js).toMatchFileSnapshot(
      `${SNAPSHOTS_DIR}/${helloWorldServiceName}/declarations/${helloWorldServiceName}.did.js.snapshot`
    );
    await expect(result.declarations_ts).toMatchFileSnapshot(
      `${SNAPSHOTS_DIR}/${helloWorldServiceName}/declarations/${helloWorldServiceName}.did.d.ts.snapshot`
    );
    await expect(result.interface_ts).toMatchFileSnapshot(
      `${SNAPSHOTS_DIR}/${helloWorldServiceName}/${helloWorldServiceName}.d.ts.snapshot`
    );
    await expect(result.service_ts).toMatchFileSnapshot(
      `${SNAPSHOTS_DIR}/${helloWorldServiceName}/${helloWorldServiceName}.ts.snapshot`
    );
  });
});
