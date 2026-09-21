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

  it('treats an omitted field named after an inherited member as absent', async () => {
    const { to_candid_Shadowed } = await conversionsFor('conversion_encoding');

    // `value.toString` on an object without that field is `Object.prototype.toString`, so a
    // presence test alone reports the inherited function as a value to encode.
    const wire = to_candid_Shadowed({ ok: 1n } as never) as Record<string, unknown[]>;

    expect(wire.toString).toEqual([]);
    expect(to_candid_Shadowed({ toString: 'x', ok: 1n } as never)).toMatchObject({
      toString: ['x'],
    });
  });

  it('keeps a present-but-falsy payload on a variant', async () => {
    const { to_candid_Payload, from_candid_Payload } = await conversionsFor('conversion_encoding');

    // The discriminated-union path is separate from the record one and needs its own cover.
    const wire = to_candid_Payload({ __kind__: 'count', count: 0n } as never) as Record<
      string,
      unknown
    >;
    expect(wire.count).toEqual([0n]);
    expect(from_candid_Payload(wire as never)).toMatchObject({ count: 0n });
  });

  it('round-trips nested optional fields through undefined, null and a value', async () => {
    const { to_candid_Settings, from_candid_Settings } =
      await conversionsFor('nested_option_fields');

    // `a?: Cfg | null`: omitted is absent, `null` is present with an absent inner value,
    // a value is present. Three levels deep the middle is a `Some`/`None` of its own.
    expect(to_candid_Settings({} as never)).toEqual({ a: [], b: [], c: [], d: [] });
    expect(to_candid_Settings({ a: null, b: 5n, c: { __kind__: 'None' } } as never)).toEqual({
      a: [[]],
      b: [[5n]],
      c: [[]],
      d: [],
    });
    expect(
      to_candid_Settings({ a: { x: 1n }, c: { __kind__: 'Some', value: null } } as never),
    ).toMatchObject({ a: [[{ x: 1n }]], c: [[[]]] });

    expect(from_candid_Settings({ a: [], b: [], c: [], d: [] } as never)).toEqual({});
    expect(from_candid_Settings({ a: [[]], b: [[7n]], c: [[]], d: [] } as never)).toEqual({
      a: null,
      b: 7n,
      c: { __kind__: 'None' },
    });
    expect(
      from_candid_Settings({ a: [[{ x: 2n }]], b: [], c: [[[{ x: 3n }]]], d: [] } as never),
    ).toEqual({
      a: { x: 2n },
      c: { __kind__: 'Some', value: { x: 3n } },
    });
  });

  // `d : opt Inner` and `a : opt opt Cfg` are one candid type, so one wire value and one
  // declared value: a name standing for the inner option changes neither.
  it('round-trips an aliased nested optional field exactly as the spelled-out one', async () => {
    const { to_candid_Settings, from_candid_Settings } =
      await conversionsFor('nested_option_fields');

    for (const value of [undefined, null, { x: 9n }]) {
      const encoded = to_candid_Settings({ a: value, d: value } as never) as {
        a: unknown;
        d: unknown;
      };
      expect(encoded.d).toEqual(encoded.a);

      const decoded = from_candid_Settings({
        a: encoded.a,
        b: [],
        c: [],
        d: encoded.d,
      } as never) as { a: unknown; d: unknown };
      expect(decoded.d).toEqual(decoded.a);
      expect(decoded.a).toEqual(value === undefined ? undefined : value);
    }
  });

  it('round-trips an option reached through a name as a nested one', async () => {
    const { to_candid_ViaAlias, from_candid_ViaAlias } = await conversionsFor('option_aliases');

    // Three states, so `Some`/`None` rather than a second `null`.
    expect(to_candid_ViaAlias({ __kind__: 'None' } as never)).toEqual([]);
    expect(to_candid_ViaAlias({ __kind__: 'Some', value: null } as never)).toEqual([[]]);
    expect(to_candid_ViaAlias({ __kind__: 'Some', value: { x: 4n } } as never)).toEqual([
      [{ x: 4n }],
    ]);

    expect(from_candid_ViaAlias([] as never)).toEqual({ __kind__: 'None' });
    expect(from_candid_ViaAlias([[]] as never)).toEqual({ __kind__: 'Some', value: null });
    expect(from_candid_ViaAlias([[{ x: 4n }]] as never)).toEqual({
      __kind__: 'Some',
      value: { x: 4n },
    });
  });

  it('keeps a present-but-falsy standalone optional', async () => {
    const { to_candid_opt } = await conversionsFor('conversion_encoding');
    expect(to_candid_opt, 'the standalone opt conversion should be extracted').toBeTypeOf(
      'function',
    );

    // A bare `opt` argument has no owning record, so it takes a path of its own.
    expect(to_candid_opt(0n as never)).toEqual([0n]);
    expect(to_candid_opt(undefined as never)).toEqual([]);
    expect(to_candid_opt(null as never)).toEqual([]);
  });

  it('reaches the global Object even when a candid type is named after it', async () => {
    const { from_candid_Object } = await conversionsFor('object_global');

    // The conversion calls `Object.prototype.hasOwnProperty`; if the candid type of that name
    // were declared unescaped, `Object` would resolve to the enum and this would throw.
    expect(from_candid_Object({ toString: null } as never)).toBe('toString');
    expect(from_candid_Object({ plain: null } as never)).toBe('plain');
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
  // Only names unique once the `_nN` suffix is dropped get the short alias; the generator
  // emits several `from_candid_opt_nN`, and collapsing those would bind whichever came last.
  const short = (name: string) => name.replace(/_n\d+$/, '');
  const counts = new Map<string, number>();
  for (const name of exported) counts.set(short(name), (counts.get(short(name)) ?? 0) + 1);
  const aliases = exported
    .map((name) => (counts.get(short(name)) === 1 ? `${short(name)}: ${name}` : name))
    .join(', ');

  const { outputText } = ts.transpileModule(`${standalone}\nreturn { ${aliases} };`, {
    compilerOptions: { target: ts.ScriptTarget.ES2023, module: ts.ModuleKind.None },
  });

  return new Function(outputText)() as Record<string, (value: never) => never>;
}
