// biome-ignore assist/source/organizeImports: vi must be imported before memfs
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({ default: memFs.promises, ...memFs.promises }));

/**
 * Interfaces of canisters people actually generate bindings for, vendored from their
 * repositories. The fixtures under `tests/assets` are small and each pins one rule; none of
 * them looks like a production interface, and a rule that refuses a legal shape shows up
 * here first. Only success is asserted: the output is large and would churn on every
 * unrelated change, and "it generates" is the whole guard.
 */

const OUTPUT_DIR = 'output';

beforeAll(async () => {
  await testWasmInit();
});
beforeEach(() => {
  vol.reset();
});

describe('ecosystem interfaces', () => {
  it.each([
    'internet_identity',
    'nns_governance',
    'ic_management',
    'sns_swap',
  ])('generates the bindings of %s', async (name) => {
    await generate({
      didFile: `./tests/assets/ecosystem/${name}.did`,
      outDir: OUTPUT_DIR,
      output: { actor: { interfaceFile: true } },
    });

    expect(vol.existsSync(`${OUTPUT_DIR}/${name}.ts`)).toBe(true);
    expect(vol.existsSync(`${OUTPUT_DIR}/${name}.d.ts`)).toBe(true);
  });
});
