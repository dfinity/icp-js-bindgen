import { beforeAll, describe, expect, it } from 'vitest';
import { wasmGenerate } from '../src/core/generate/rs.ts';
import { testWasmInit } from './utils/wasm.ts';

const DID_FILE = './tests/assets/reserved_words.did';

/**
 * Regression tests for reserved-word candid tags.
 *
 * All-null variants are lowered to a TypeScript `enum`, the one place where a candid tag
 * becomes a TypeScript identifier. Escaping the member declaration (`new_`) while
 * referencing the tag verbatim (`.new`) produced references to members that do not exist,
 * so decoding yielded `undefined` and encoding compared against `undefined` — silently,
 * because the generated wrapper ships with `@ts-nocheck`.
 *
 * See https://github.com/dfinity/icp-js-bindgen/issues/151.
 */
describe('reserved-word candid tags', () => {
  let serviceTs: string;
  let interfaceTs: string;

  beforeAll(async () => {
    await testWasmInit();

    const result = wasmGenerate({
      did_file_path: DID_FILE,
      service_name: 'reserved_words',
      declarations: { root_exports: false },
    });
    serviceTs = result.service_ts;
    interfaceTs = result.interface_ts;
  });

  it('declares enum members using the candid tag verbatim', () => {
    // Reserved words are valid enum member names, so no trailing underscore.
    expect(serviceTs).toContain('new = "new"');
    expect(serviceTs).toContain('default = "default"');
    expect(serviceTs).toContain('in = "in"');
    expect(serviceTs).toContain('class = "class"');
    expect(serviceTs).toContain('function = "function"');

    expect(serviceTs).not.toContain('new_ = "new"');
    expect(serviceTs).not.toContain('default_ = "default"');
  });

  it('references only enum members that were declared', () => {
    // Every `Enum.member` reference must resolve to a declared member. Collect the
    // declared members per enum, then check each reference against them.
    const declared = new Map<string, Set<string>>();
    for (const [, name, body] of serviceTs.matchAll(/enum (\w+) \{([^}]*)\}/g)) {
      const members = new Set(
        [...body.matchAll(/^\s*([A-Za-z_$][\w$]*|'[^']*') = /gm)].map((m) => m[1]),
      );
      declared.set(name, members);
    }
    expect(declared.size).toBeGreaterThan(0);

    for (const [, enumName, member] of serviceTs.matchAll(/\b(\w+)\.([A-Za-z_$][\w$]*)\b/g)) {
      const members = declared.get(enumName);
      if (!members) continue; // not an enum access
      expect(members, `${enumName}.${member} is referenced but not declared`).toContain(member);
    }
  });

  it('encodes the candid field name verbatim, not the escaped identifier', () => {
    // The object literal handed to the agent is a candid value: `{ new: null }`.
    // Emitting `{ new_: null }` would encode a field the candid type does not have.
    expect(serviceTs).toMatch(/\{\s*new: null\s*\}/);

    // Each encode branch compares against an enum member and emits the corresponding
    // candid field. Since members are now the tag verbatim, the two must be identical.
    // (`{ new_: null }` is legitimate here — `Collide` really has a tag named `new_`.)
    const branches = [...serviceTs.matchAll(/value == (\w+)\.([\w$]+) \? \{\s*([\w$]+): null/g)];
    expect(branches.length).toBeGreaterThan(0);
    for (const [, enumName, member, field] of branches) {
      expect(field, `${enumName}.${member} must encode field "${member}", got "${field}"`).toBe(
        member,
      );
    }
  });

  it('escapes an enum type name that is a reserved word, consistently', () => {
    // `type new = variant { a; b }` — the type name must be escaped (an identifier cannot
    // be `new`) and every reference must use that same escaped name. Referencing it as
    // `new.a` is a syntax error, not merely a type error.
    expect(serviceTs).toContain('enum new_');
    expect(serviceTs).not.toMatch(/\bnew\.[ab]\b/);
    expect(serviceTs).not.toMatch(/:\s*new\b(?!_)/);
  });

  it('does not emit duplicate members when escaping would collide', () => {
    // `variant { new; new_ }`: naive escaping turns `new` into `new_`, colliding with the
    // real `new_` tag and declaring the same member twice.
    const collide = serviceTs.match(/enum (?:\w*Collide\w*|Variant_new_new_) \{([^}]*)\}/);
    if (!collide) {
      throw new Error(`expected a Collide enum in the generated output:\n${serviceTs}`);
    }
    const members = [...collide[1].matchAll(/^\s*([\w$']+) = /gm)].map((m) => m[1]);
    expect(members).toEqual([...new Set(members)]);
    expect(members).toEqual(expect.arrayContaining(['new', 'new_']));
  });

  it('imports the escaped name that the declarations file actually exports', () => {
    // The declarations layer escapes exported type names (`export type new_`), so the
    // wrapper must import `new_`; importing `new` names a member that does not exist.
    expect(serviceTs).toMatch(/import type \{[^}]*\bnew_ as _new\b/);
    expect(serviceTs).not.toMatch(/\bnew as _new\b/);
  });

  it('applies the same treatment to the interface declaration file', () => {
    expect(interfaceTs).toContain('new = "new"');
    expect(interfaceTs).not.toContain('new_ = "new"');
  });
});
