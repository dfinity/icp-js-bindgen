import { describe, it, expect, beforeAll, beforeEach, vi } from "vitest";
import { fs as memFs, vol } from "memfs";
import fs from "node:fs/promises";
import { generate } from "../src/core/generate/index.ts";
import { testWasmInit } from "./utils/wasm.ts";

vi.mock("node:fs/promises", () => ({
  default: memFs.promises,
  ...memFs.promises,
}));

const TESTS_ASSETS_DIR = "./tests/assets";
const OUTPUT_DIR = "output";

beforeAll(async () => {
  await testWasmInit();
});

beforeEach(() => {
  vol.reset();
});

describe("generate", () => {
  it("should generate a bindgen", async () => {
    const helloWorldServiceName = "hello_world";
    const helloWorldDidFile = `${TESTS_ASSETS_DIR}/${helloWorldServiceName}.did`;

    await generate({ didFile: helloWorldDidFile, outDir: OUTPUT_DIR });

    expect(await fs.readdir(OUTPUT_DIR)).toEqual([
      "declarations",
      "hello_world.d.ts",
      "hello_world.ts",
      "index.ts",
    ]);
  });
});
