// biome-ignore assist/source/organizeImports: vi must be imported before memfs
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({
  default: memFs.promises,
  ...memFs.promises,
}));

/**
 * The generator emits TypeScript it never re-reads, and the generated files carry
 * `// @ts-nocheck`, so a malformed or inconsistent module reaches the user's build rather
 * than failing here. These tests pin the cases where generation must refuse instead.
 *
 * The complementary assertion — that the checks reject nothing the generator legitimately
 * emits — is the rest of the suite staying green, and in particular the snapshot fixtures
 * that `tests/typecheck.test.ts` runs `tsc` over.
 */

const TESTS_ASSETS_DIR = './tests/assets';
const OUTPUT_DIR = 'output';

beforeAll(async () => {
  await testWasmInit();
});

beforeEach(() => {
  vol.reset();
});

describe('generation refuses invalid output', () => {
  it('reports a candid type that collides with the generated class name', async () => {
    await expect(generateFixture('collide_class')).rejects.toThrow(/Collide_class/);
  });

  it('reports two variant tag sets that sanitize to the same enum name', async () => {
    // Sanitizing tags into an identifier is what makes these collide. Nothing downstream
    // would report it: the members are disjoint, so TypeScript merges the two enums silently
    // and the merged type admits members neither candid variant has.
    await expect(generateFixture('collide_variant_tags')).rejects.toThrow(/Variant_my_f/);
  });

  it('reports two candid types that escape to the same enum name', async () => {
    // `Map` is escaped to `Map_` because it shadows a global, which collides with a candid
    // type actually named `Map_`. Reusing one enum for both would leave the second type's
    // members undeclared while every reference still resolved, so nothing downstream sees it.
    await expect(generateFixture('collide_escaped_names')).rejects.toThrow(/Map_/);
  });

  it('names both colliding declaration kinds, so the cause is actionable', async () => {
    await expect(generateFixture('collide_class')).rejects.toThrow(
      /interface.*class|class.*interface/s,
    );
  });
});

async function generateFixture(serviceName: string): Promise<void> {
  await generate({
    // The wasm reads the .did file from the real filesystem, not from memfs.
    didFile: `${TESTS_ASSETS_DIR}/${serviceName}.did`,
    outDir: OUTPUT_DIR,
    output: { actor: { interfaceFile: true } },
  });
}
