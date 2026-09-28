import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { testWasmInit } from './utils/wasm.ts';
import { loadWrapper, type RawActor } from './utils/wrapper.ts';

let cleanup: (() => void) | undefined;

beforeAll(async () => {
  await testWasmInit();
});

afterEach(() => {
  cleanup?.();
  cleanup = undefined;
});

const actor: RawActor = { get: async () => 42n };

async function load(serviceName: string) {
  const loaded = await loadWrapper(serviceName, actor);
  cleanup = loaded.cleanup;
  return loaded;
}

describe('actor class name', () => {
  it('does not merge the actor class with a candid type of the same name', async () => {
    const { wrapper, source } = await load('governance');

    expect(await wrapper.get()).toBe(42n);
    // A class and an interface of one name merge in TypeScript, so the class would claim the
    // record's fields. The candid type keeps its name; the class steps aside.
    expect(source).toContain('export interface Governance {');
    expect(source).toContain('export class Governance_ ');
    expect(source).toContain('): Governance_ {');
  });
});
