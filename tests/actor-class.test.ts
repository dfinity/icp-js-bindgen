// biome-ignore assist/source/organizeImports: vi must be imported before memfs
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({ default: memFs.promises, ...memFs.promises }));

const ASSETS_DIR = './tests/assets/actor_class';
const OUTPUT_DIR = 'output';

// The .did files are read from disk; only the output goes to memfs.
let renamedDir: string;

beforeAll(async () => {
  await testWasmInit();
  renamedDir = mkdtempSync(join(tmpdir(), 'icp-bindgen-actor-class-'));
});

afterAll(() => {
  rmSync(renamedDir, { recursive: true, force: true });
});

beforeEach(() => {
  vol.reset();
});

/** Every name the wrapper generated for `didFile` binds at the top level. */
async function topLevelNames(didFile: string, serviceName: string): Promise<string[]> {
  await generate({ didFile, outDir: OUTPUT_DIR });
  const wrapper = String(vol.readFileSync(`${OUTPUT_DIR}/${serviceName}.ts`, 'utf-8'));
  const imported = [...wrapper.matchAll(/^import (?:type )?\{([^}]*)\}/gm)].flatMap(([, names]) =>
    names.split(',').map((name) => (name.trim().split(/\s+/).at(-1) ?? '').trim()),
  );
  const declared = [
    ...wrapper.matchAll(/^(?:export )?(?:interface|type|class|enum|function|const) (\w+)/gm),
  ].map(([, name]) => name);
  return [...new Set([...imported, ...declared])];
}

describe('actor class name collisions', () => {
  it('rejects a file named after anything its wrapper binds', async () => {
    const source = `${ASSETS_DIR}/names.did`;
    // A class name starts with an uppercase letter or `_`, so only those can collide.
    const names = (await topLevelNames(source, 'names')).filter(
      (name) => /^[A-Z_]/.test(name) && name !== 'Names',
    );
    for (const expected of ['Rec', 'Alias', 'SvcAlias', 'SvcInterface', 'Variant_a_b', '_Rec']) {
      expect(names).toContain(expected);
    }
    expect(names).toContain('_SERVICE');
    expect(names).toContain('Option');

    for (const name of names) {
      const didFile = join(renamedDir, `${name}.did`);
      writeFileSync(didFile, readFileSync(source));
      await expect(generate({ didFile, outDir: OUTPUT_DIR }), name).rejects.toThrow(
        `The actor class \`${name}\`, named after ${name}.did, has the same name as`,
      );
    }
  });

  it('rejects NNS governance under its upstream file name', async () => {
    await expect(
      generate({ didFile: `${ASSETS_DIR}/governance.did`, outDir: OUTPUT_DIR }),
    ).rejects.toThrow(
      'The actor class `Governance`, named after governance.did, has the same name as an interface in the generated wrapper. Rename the .did file',
    );
  });

  it('generates a file named after a candid service type', async () => {
    // The service type is declared as `FooInterface`, so nothing else is named `Foo`.
    await generate({ didFile: `${ASSETS_DIR}/foo.did`, outDir: OUTPUT_DIR });

    expect(vol.existsSync(`${OUTPUT_DIR}/foo.ts`)).toBe(true);
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
