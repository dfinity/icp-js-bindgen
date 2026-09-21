// biome-ignore assist/source/organizeImports: vi must be imported before memfs
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { generate } from '../src/core/generate/index.ts';
import { setupTypecheckDir, stripTsNocheck, typecheckFiles } from './utils/typecheck.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({ default: memFs.promises, ...memFs.promises }));

/**
 * Interfaces of canisters people actually generate bindings for, vendored from their
 * repositories. The fixtures under `tests/assets` are small and each pins one rule; none of
 * them looks like a production interface, and a rule that refuses a legal shape shows up
 * here first. Generation must succeed, and the wrapper must typecheck with its
 * `// @ts-nocheck` header stripped; the output itself is not snapshotted, since it is large
 * and would move on every unrelated change.
 */

const OUTPUT_DIR = 'output';

/**
 * Diagnostics the generator is known to produce today, by TypeScript error code. An entry here
 * is a bug with an owner, not an accepted state: delete it when the mapping is fixed.
 */
const KNOWN_DIAGNOSTICS: Record<string, number[]> = {};

let tmpDir: string;

beforeAll(async () => {
  await testWasmInit();
  tmpDir = setupTypecheckDir();
});
beforeEach(() => {
  vol.reset();
});
afterAll(() => {
  if (tmpDir) rmSync(tmpDir, { recursive: true, force: true });
});

describe('ecosystem interfaces', () => {
  it.each([
    'internet_identity',
    'nns_governance',
    'ic_management',
    'sns_swap',
  ])('generates typechecked bindings for %s', async (name) => {
    await generate({
      didFile: `./tests/assets/ecosystem/${name}.did`,
      outDir: OUTPUT_DIR,
      output: { actor: { interfaceFile: true } },
    });

    const caseDir = join(tmpDir, `ecosystem_${name}`);
    mkdirSync(join(caseDir, 'declarations'), { recursive: true });
    const wrapperPath = join(caseDir, `${name}.ts`);
    const declarationsPath = join(caseDir, 'declarations', `${name}.did.d.ts`);
    writeFileSync(
      wrapperPath,
      stripTsNocheck(vol.readFileSync(`${OUTPUT_DIR}/${name}.ts`, 'utf-8') as string),
    );
    writeFileSync(
      declarationsPath,
      vol.readFileSync(`${OUTPUT_DIR}/declarations/${name}.did.d.ts`, 'utf-8') as string,
    );

    const codes = typecheckFiles(tmpDir, [wrapperPath, declarationsPath])
      .map((d) => d.code)
      .sort();
    expect(codes).toEqual(KNOWN_DIAGNOSTICS[name] ?? []);
  });
});
