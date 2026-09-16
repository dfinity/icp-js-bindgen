// biome-ignore assist/source/organizeImports: vi must be imported before memfs
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({ default: memFs.promises, ...memFs.promises }));

/**
 * Evaluates the generated `idlFactory` against a recording stub, so the record shapes it
 * builds can be inspected.
 *
 * A field name that an object literal treats specially never reaches the schema at all, and
 * nothing downstream can notice: encoding and decoding simply act as though the candid type
 * had no such field.
 */

const OUTPUT_DIR = 'output';

beforeAll(async () => {
  await testWasmInit();
});
beforeEach(() => {
  vol.reset();
});

describe('generated IDL schema', () => {
  it('registers a field named __proto__', async () => {
    const records = await recordsFor('conversion_encoding');

    const proto = records.find((fields) => 'ok' in fields);
    expect(proto, 'the Proto record should be built').toBeDefined();
    expect(Object.hasOwn(proto as object, '__proto__')).toBe(true);
  });
});

/** Every `IDL.Record({…})` the factory builds, as plain key sets. */
async function recordsFor(serviceName: string): Promise<Record<string, unknown>[]> {
  await generate({ didFile: `./tests/assets/${serviceName}.did`, outDir: OUTPUT_DIR });
  const source = await readFile(
    resolve(OUTPUT_DIR, 'declarations', `${serviceName}.did.js`),
    'utf-8',
  );

  const records: Record<string, unknown>[] = [];
  const marker = () => ({});
  const IDL = new Proxy(
    {
      Record: (fields: Record<string, unknown>) => {
        records.push(fields);
        return marker();
      },
    } as Record<string, unknown>,
    {
      // Every other IDL constructor is irrelevant here, so answer with a callable stub.
      get: (target, key) =>
        key in target ? target[key as string] : Object.assign(marker, { fill: marker }),
    },
  );

  // The stub is passed in, so the real import must go — and nothing here is a module.
  const body = source
    .split('\n')
    .filter((line) => !line.startsWith('import '))
    .join('\n')
    .replace(/^export const/gm, 'const');
  new Function('IDL', `${body}\nidlFactory({ IDL });`)(IDL);
  return records;
}
