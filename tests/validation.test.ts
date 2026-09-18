// biome-ignore assist/source/organizeImports: vi must be imported before memfs
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { resolve } from 'node:path';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({ default: memFs.promises, ...memFs.promises }));

/**
 * The generator emits TypeScript it never re-reads, and the generated files carry
 * `// @ts-nocheck`, so an inconsistent module reaches the user's build rather than failing
 * here. These pin the cases where generation must refuse instead.
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

describe('generation refuses inconsistent output', () => {
  it('reports an inline variant whose enum takes the generated class name', async () => {
    // `variant_a_b.did` derives the class `Variant_a_b`; so does the inline `variant { a; b }`.
    // A candid *type* of that name is escaped up front; an enum named while the module is
    // built cannot be, so this is the case the checks exist for.
    await expect(generateFixture('variant_a_b')).rejects.toThrow(/Variant_a_b/);
  });

  it('names both colliding kinds, so the cause is actionable', async () => {
    await expect(generateFixture('variant_a_b')).rejects.toThrow(/enum.*class|class.*enum/s);
  });

  // JavaScript treats `__proto__` as the prototype in every position it would appear: a
  // record key, an enum member, a method — in the declarations as much as in the actor files,
  // since `IDL.Record({ '__proto__': … })` loses the field too. Refused ahead of every
  // generator, whichever files are wanted.
  it.each([
    'unrepresentable_tag',
    'unrepresentable_field',
    'unrepresentable_nested_method',
  ])('refuses a candid name of __proto__: %s', async (fixture) => {
    await expect(generateFixture(fixture)).rejects.toThrow(/__proto__/);
  });

  it('refuses __proto__ even when only the declarations are wanted', async () => {
    await expect(
      generate({
        didFile: `${TESTS_ASSETS_DIR}/unrepresentable_field.did`,
        outDir: OUTPUT_DIR,
        output: { actor: { disabled: true } },
      }),
    ).rejects.toThrow(/__proto__/);
  });

  it('reports two variant tag sets that sanitize to the same enum name', async () => {
    // Sanitizing tags into an identifier is what makes these collide. Their members are
    // disjoint, so TypeScript would merge the two enums and the merged type would accept a
    // member neither candid variant has.
    await expect(generateFixture('collide_variant_tags')).rejects.toThrow(/Variant_my_f/);
  });

  it('reports two candid types that escape to the same name', async () => {
    // `Map` is escaped because it shadows a global, which collides with a candid type
    // actually named `Map_`. One enum cannot carry both tag lists, and reusing one would
    // leave the other's members undeclared while every reference still resolved.
    await expect(generateFixture('collide_escaped_names')).rejects.toThrow(/Map_/);
  });
  it('reports a candid type that collides with a generated conversion function', async () => {
    await expect(generateFixture('collide_conversion_function')).rejects.toThrow(
      /to_candid_foo_n1/,
    );
  });

  it('reports two escaped names that collide even with identical tags', async () => {
    // Identical tags would let one enum serve both, which is a merge no check can see.
    await expect(generateFixture('collide_escaped_same_tags')).rejects.toThrow(/Map_/);
  });

  it('reports two inline variants whose tags join to one enum name', async () => {
    // `variant { a_b }` and `variant { a; b }` both derive `Variant_a_b` with disjoint
    // members, which TypeScript would merge into an enum accepting a member neither has.
    await expect(generateFixture('collide_anonymous_variant_names')).rejects.toThrow(/Variant_a_b/);
  });

  it('reports a candid tag that collides with the injected discriminant', async () => {
    // A variant carrying payloads gains a `__kind__` discriminant, so a candid tag of that
    // name lands twice in the same object type and the later member wins.
    await expect(generateFixture('duplicate_member')).rejects.toThrow(/__kind__/);
  });

  // The module checks describe the actor files. A caller who only wants `declarations/` is
  // not affected by a collision in the wrapper.
  it('still generates declarations when the actor files are not wanted', async () => {
    await generate({
      didFile: `${TESTS_ASSETS_DIR}/variant_a_b.did`,
      outDir: OUTPUT_DIR,
      output: { actor: { disabled: true } },
    });

    expect(vol.existsSync(resolve(OUTPUT_DIR, 'declarations', 'variant_a_b.did.d.ts'))).toBe(true);
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
