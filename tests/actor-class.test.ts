// biome-ignore assist/source/organizeImports: vi must be imported before memfs
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({ default: memFs.promises, ...memFs.promises }));

const ASSETS_DIR = './tests/assets/actor_class';
const OUTPUT_DIR = 'output';

beforeAll(async () => {
  await testWasmInit();
});

beforeEach(() => {
  vol.reset();
});

describe('actor class name collisions', () => {
  it.each([
    ['governance', 'Governance', 'the candid type `Governance`'],
    ['fooInterface', 'FooInterface', 'the candid service type `Foo`'],
    ['some', 'Some', 'the generated `Some` interface'],
  ])('rejects %s.did', async (serviceName, className, collision) => {
    const generating = generate({
      didFile: `${ASSETS_DIR}/${serviceName}.did`,
      outDir: OUTPUT_DIR,
    });

    await expect(generating).rejects.toThrow(`The actor class \`${className}\``);
    await expect(generating).rejects.toThrow(collision);
    await expect(generating).rejects.toThrow('Rename the .did file');
  });

  it('still generates the declarations without the actor output', async () => {
    await generate({
      didFile: `${ASSETS_DIR}/governance.did`,
      outDir: OUTPUT_DIR,
      output: { actor: { disabled: true } },
    });

    expect(vol.existsSync(`${OUTPUT_DIR}/declarations/governance.did.d.ts`)).toBe(true);
    expect(vol.existsSync(`${OUTPUT_DIR}/governance.ts`)).toBe(false);
  });
});
