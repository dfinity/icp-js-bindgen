// biome-ignore assist/source/organizeImports: vi must be imported before memfs and memfs must be imported before readFile
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import ts from 'typescript';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({
  default: memFs.promises,
  ...memFs.promises,
}));

/**
 * Guards the identifiers the generator emits in *binding* position — declaration names and
 * type references — against being quoted rather than sanitized.
 *
 * Quoting is correct in property position (`'my-field': bigint` is legal) and a syntax
 * error in binding position (`interface 'my-field'` is not), and the two used to share a
 * code path. Every fixture here is a name that took the wrong branch: see
 * https://github.com/dfinity/icp-js-bindgen/issues/156
 *
 * The wrapper ships with a `// @ts-nocheck` header, which suppresses type errors but not
 * parse errors. Both are checked here: the header is stripped, and the file is parsed.
 */

const TESTS_ASSETS_DIR = './tests/assets';
const OUTPUT_DIR = 'output';

/**
 * The identifier shape this test enforces — deliberately *looser* than the generator's own
 * rule, so it checks "TypeScript would accept this" rather than restating the
 * implementation.
 *
 * Modelled on ECMAScript `IdentifierName` (`ID_Start`/`ID_Continue` plus `$` and `_`),
 * omitting the zero-width joiners and unicode escape sequences the grammar also permits but
 * that the generator never emits. The generator restricts itself to `XID_*`, which is a
 * subset — so output that satisfies its rule always satisfies this one, and a regression
 * that emitted a quoted or otherwise malformed name would still be caught here.
 */
const ENFORCED_IDENTIFIER_SHAPE = /^[\p{ID_Start}$_][\p{ID_Continue}$]*$/u;

/**
 * Captures the name of every declaration the generator emits: `class`, `interface`, `enum`
 * and `type`. All four put the name in binding position, so all four must be checked —
 * `type` included, since Candid variants with a payload become type aliases.
 */
const DECLARED_IDENTIFIER =
  /^\s*(?:export\s+)?(?:declare\s+)?(?:class|interface|enum|type)\s+(\S+?)(?:<|\s|=|$)/gm;

/**
 * Service names that are not legal identifiers, or that become a reserved word once
 * capitalized for the class name.
 */
const AWKWARD_SERVICE_NAMES = ['my-backend', 'map'];

/** Service names whose *Candid* contents, rather than filename, produce awkward names. */
const AWKWARD_CANDID_NAMES = ['variant_special_tags'];

beforeAll(async () => {
  await testWasmInit();
});

beforeEach(() => {
  vol.reset();
});

describe('identifiers in binding position', () => {
  it.each([
    ...AWKWARD_SERVICE_NAMES,
    ...AWKWARD_CANDID_NAMES,
  ])('emits parseable TypeScript for %s', async (serviceName) => {
    await generateFixture(serviceName);

    for (const file of [`${serviceName}.ts`, `${serviceName}.d.ts`]) {
      const source = await readFileFromOutput(file);
      expect(syntaxErrorsIn(source), `${file} should parse`).toEqual([]);
    }
  });

  it.each([
    ...AWKWARD_SERVICE_NAMES,
    ...AWKWARD_CANDID_NAMES,
  ])('declares only legal identifiers for %s', async (serviceName) => {
    await generateFixture(serviceName);

    for (const file of [`${serviceName}.ts`, `${serviceName}.d.ts`]) {
      const source = await readFileFromOutput(file);
      const declared = [...source.matchAll(DECLARED_IDENTIFIER)].map(([, name]) => name);

      // Guards against the regex silently matching nothing and the assertion passing.
      expect(declared.length, `${file} should declare something`).toBeGreaterThan(0);
      for (const name of declared) {
        expect(name, `${file} declares ${name}`).toMatch(ENFORCED_IDENTIFIER_SHAPE);
      }
    }
  });

  // The import specifier used to rewrite `-` to `_` while the file kept the dash, so it
  // pointed at a module that was never written. Parsing the specifier out of the generated
  // file (rather than reconstructing it from the service name) is what makes this a real
  // check: reconstructing it would assume exactly what the bug got wrong.
  it.each(AWKWARD_SERVICE_NAMES)('imports declarations that exist for %s', async (serviceName) => {
    await generateFixture(serviceName);

    for (const file of [`${serviceName}.ts`, `${serviceName}.d.ts`]) {
      const source = await readFileFromOutput(file);
      const specifiers = [...source.matchAll(/from ["'](\.[^"']+)["']/g)].map(([, s]) => s);

      expect(specifiers.length, `${file} should import declarations`).toBeGreaterThan(0);
      for (const specifier of specifiers) {
        // Imports are extensionless, so any of the declaration extensions may satisfy it.
        const candidates = ['.js', '.d.ts', '.ts'].map((ext) =>
          resolve(OUTPUT_DIR, `${specifier}${ext}`),
        );
        expect(
          candidates.some((candidate) => vol.existsSync(candidate)),
          `${file} imports "${specifier}", which matches no generated file`,
        ).toBe(true);
      }
    }
  });

  it('escapes a class name that collides with a global, at every reference', async () => {
    // `map.did` -> `Map`, which is the global `Map`. The declaration was escaped but the
    // `createActor` return type and the `new …()` call were not, so `createActor` returned
    // a real `Map` with none of the service methods.
    await generateFixture('map');
    const source = await readFileFromOutput('map.ts');

    expect(source).toContain('export class Map_ ');
    expect(source).toContain('): Map_ {');
    expect(source).toContain('return new Map_(actor);');
    expect(source).not.toMatch(/\bnew Map\(/);
    expect(source).not.toMatch(/\):\s*Map\s*\{/);
  });

  it('sanitizes a service name that is not a legal identifier', async () => {
    await generateFixture('my-backend');
    const source = await readFileFromOutput('my-backend.ts');

    // `my-backend` yields exactly what `my_backend.did` would have.
    expect(source).toContain('export interface my_backendInterface {');
    expect(source).toContain('export class My_backend implements my_backendInterface {');
    expect(source).toContain('from "./declarations/my-backend.did"');
  });

  it('sanitizes an enum name derived from candid tags', async () => {
    await generateFixture('variant_special_tags');
    const source = await readFileFromOutput('variant_special_tags.ts');

    // The tags stay quoted as enum *members* — legal there — while the enum name, which is
    // an identifier, is sanitized.
    expect(source).toMatch(/export enum Variant_\w+ \{/);
    expect(source).toContain("'my-tag'");
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

async function readFileFromOutput(path: string): Promise<string> {
  return await readFile(resolve(OUTPUT_DIR, path), 'utf-8');
}

/**
 * Parse-level diagnostics only. `transpileModule` does not resolve modules or check types,
 * so this reports malformed syntax without needing the `@icp-sdk/core` packages present.
 */
function syntaxErrorsIn(source: string): string[] {
  const withoutNoCheck = source
    .split('\n')
    .filter((line) => line.trim() !== '// @ts-nocheck')
    .join('\n');

  const { diagnostics = [] } = ts.transpileModule(withoutNoCheck, {
    reportDiagnostics: true,
    compilerOptions: {
      target: ts.ScriptTarget.ES2023,
      module: ts.ModuleKind.ESNext,
      isolatedModules: false,
    },
  });

  return diagnostics.map(
    (d) => `TS${d.code}: ${ts.flattenDiagnosticMessageText(d.messageText, ' ')}`,
  );
}
