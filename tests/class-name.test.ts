// biome-ignore assist/source/organizeImports: vi must be imported before memfs
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({ default: memFs.promises, ...memFs.promises }));

/**
 * The actor class name is derived from the `.did` basename in three places — the class
 * declaration, the `createActor` return type, and the `new …()` call. They used to be derived
 * independently, and only the declaration was escaped against globals.
 */

const OUTPUT_DIR = 'output';

beforeAll(async () => {
  await testWasmInit();
});
beforeEach(() => {
  vol.reset();
});

describe('actor class name', () => {
  it('escapes a name that collides with a global, at every reference', async () => {
    // `map.did` capitalizes to `Map`. The declaration was escaped but the return type and the
    // constructor call were not, so `createActor` handed back a real JavaScript `Map` with
    // none of the service methods — no syntax error, and `@ts-nocheck` hid the type error.
    await generate({
      didFile: './tests/assets/map.did',
      outDir: OUTPUT_DIR,
      output: { actor: { interfaceFile: true } },
    });
    const source = await readFile(resolve(OUTPUT_DIR, 'map.ts'), 'utf-8');

    expect(source).toContain('export class Map_ ');
    expect(source).toContain('): Map_ {');
    expect(source).toContain('return new Map_(actor);');
    expect(source).not.toMatch(/\bnew Map\(/);
    expect(source).not.toMatch(/\):\s*Map\s*\{/);
  });
});
