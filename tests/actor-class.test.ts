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
const HELLO_WORLD = './tests/assets/hello_world.did';

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

describe('actor class name collisions', () => {
  it.each([
    ['governance', 'Governance', 'the candid type `Governance`'],
    ['fooInterface', 'FooInterface', 'the candid service type `Foo`'],
    ['bar', 'Bar', 'the candid type `Bar`'],
    ['alias', 'Alias', 'the candid type `Alias`'],
  ])('rejects %s.did', async (serviceName, className, collision) => {
    const generating = generate({
      didFile: `${ASSETS_DIR}/${serviceName}.did`,
      outDir: OUTPUT_DIR,
    });

    await expect(generating).rejects.toThrow(`The actor class \`${className}\``);
    await expect(generating).rejects.toThrow(collision);
    await expect(generating).rejects.toThrow('Rename the .did file');
  });

  it('rejects every capitalized name the wrapper declares for itself', async () => {
    // Derived from generated output, so a name added to the preamble must be added to the check.
    await generate({ didFile: HELLO_WORLD, outDir: OUTPUT_DIR });
    const wrapper = String(vol.readFileSync(`${OUTPUT_DIR}/hello_world.ts`, 'utf-8'));
    const imported = [...wrapper.matchAll(/^import (?:type )?\{([^}]*)\}/gm)].flatMap(([, names]) =>
      names.split(',').map((name) => name.replace(/^\s*type\s+/, '').trim()),
    );
    const declared = [
      ...wrapper.matchAll(/^(?:export )?(?:interface|type|class|enum|function|const) (\w+)/gm),
    ].map(([, name]) => name);
    const names = [...new Set([...imported, ...declared])].filter(
      (name) => /^[A-Z]/.test(name) && name !== 'Hello_world',
    );
    expect(names).toContain('Option');

    for (const name of names) {
      const didFile = join(renamedDir, `${name}.did`);
      writeFileSync(didFile, readFileSync(HELLO_WORLD));
      await expect(generate({ didFile, outDir: OUTPUT_DIR }), name).rejects.toThrow(
        `The actor class \`${name}\``,
      );
    }
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
