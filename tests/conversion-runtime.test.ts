// biome-ignore assist/source/organizeImports: vi must be imported before memfs
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import ts from 'typescript';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({ default: memFs.promises, ...memFs.promises }));

/**
 * Runs the generated conversion functions.
 *
 * Every other suite checks what the generator *writes* — that it parses, typechecks, or
 * matches a snapshot. None of them notice when the emitted code is well-formed and does the
 * wrong thing, which is how a truthiness test on optional fields and a prototype-walking `in`
 * both survived: the output looked fine and encoded the wrong values.
 *
 * The wrapper's conversion functions are module-local, so they are extracted and evaluated
 * here rather than imported. The class and `createActor` are dropped because they would pull
 * in `@icp-sdk/core`.
 */

const OUTPUT_DIR = 'output';

beforeAll(async () => {
  await testWasmInit();
});
beforeEach(() => {
  vol.reset();
});

describe('generated conversions, executed', () => {
  it('keeps present-but-falsy optional fields', async () => {
    const { to_candid_Falsy, from_candid_Falsy } = await conversionsFor('conversion_encoding');

    const wire = to_candid_Falsy({ zero: 0n, no: false, empty: '' } as never) as Record<
      string,
      unknown[]
    >;

    // `[]` is candid's absent; `[v]` is present. A truthiness test dropped all three.
    expect(wire.zero).toEqual([0n]);
    expect(wire.no).toEqual([false]);
    expect(wire.empty).toEqual(['']);
    expect(wire.missing).toEqual([]);

    expect(from_candid_Falsy(wire as never)).toEqual({ zero: 0n, no: false, empty: '' });
  });

  it('keeps a record field named __proto__ as an own property', async () => {
    const { to_candid_Proto, from_candid_Proto } = await conversionsFor('conversion_encoding');

    // A computed key in the *input* too: `{ __proto__: 7n }` would set the prototype here
    // rather than create the field, which is the same trap the generator has to avoid.
    const input = { ['__proto__']: 7n, ok: 1n };
    const wire = to_candid_Proto(input as never) as Record<string, unknown>;

    // A bare or quoted key would have set the prototype and dropped the field instead.
    expect(Object.hasOwn(wire, '__proto__')).toBe(true);
    expect(wire['__proto__']).toEqual([7n]);
    expect(Object.getPrototypeOf(wire)).toBe(Object.prototype);

    const back = from_candid_Proto(wire as never) as Record<string, unknown>;
    expect(Object.hasOwn(back, '__proto__')).toBe(true);
    expect(back['__proto__']).toBe(7n);
  });

  it('does not match variant tags inherited from Object.prototype', async () => {
    const { from_candid_Inherited } = await conversionsFor('conversion_encoding');

    // `"constructor" in {plain: null}` is true, so the first such tag used to win outright.
    expect(from_candid_Inherited({ plain: null } as never)).toBe('plain');
    expect(from_candid_Inherited({ constructor: null } as never)).toBe('constructor');
    expect(from_candid_Inherited({ toString: null } as never)).toBe('toString');
  });
});

/**
 * Generates `serviceName`, then evaluates the parts of the wrapper that stand alone: the
 * preamble helpers, the enums, and the `to_candid_*` / `from_candid_*` functions.
 */
async function conversionsFor(
  serviceName: string,
): Promise<Record<string, (value: never) => never>> {
  await generate({ didFile: `./tests/assets/${serviceName}.did`, outDir: OUTPUT_DIR });
  const source = await readFile(resolve(OUTPUT_DIR, `${serviceName}.ts`), 'utf-8');

  const standalone = source
    .split(/^(?=(?:export )?(?:function|class|interface|enum|type|import|const) )/m)
    .filter((block) => !/^(import |export class |export function createActor)/.test(block))
    .join('')
    // Nothing is a module here, so `export` would compile to an `exports` assignment.
    .replace(/^export /gm, '');

  const exported = [...standalone.matchAll(/^function ((?:to|from)_candid_\w+)/gm)].map(
    ([, name]) => name,
  );
  const aliases = exported.map((name) => `${name.replace(/_n\d+$/, '')}: ${name}`).join(', ');

  const { outputText } = ts.transpileModule(`${standalone}\nreturn { ${aliases} };`, {
    compilerOptions: { target: ts.ScriptTarget.ES2023, module: ts.ModuleKind.None },
  });

  return new Function(outputText)() as Record<string, (value: never) => never>;
}
