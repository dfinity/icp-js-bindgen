import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { generate } from '../../src/core/generate/index.ts';

const ACTOR_KEY = '__icpBindgenTestActor';

// Just enough of @icp-sdk/core for the generated modules to load: `Actor.createActor` hands
// back the test's actor, and no agent or IDL value is ever used.
const CORE_STUBS = {
  'agent.js': [
    `export const Actor = { createActor: () => globalThis.${ACTOR_KEY} };`,
    'export const HttpAgent = { createSync: () => ({}) };',
    '',
  ].join('\n'),
  'candid.js': [
    'const any = new Proxy(function () {}, {',
    "  get: (_, prop) => (prop === 'then' ? undefined : any),",
    '  apply: () => any,',
    '});',
    'export const IDL = any;',
    '',
  ].join('\n'),
  'principal.js': 'export class Principal {}\n',
};

export type RawActor = Record<string, (...args: unknown[]) => Promise<unknown>>;
export type Wrapper = Record<string, (...args: unknown[]) => Promise<unknown>>;

/**
 * Generates the bindings for `tests/assets/<serviceName>.did` and calls their `createActor`
 * with `actor` as the underlying actor, so a test sees what the wrapper sends to and returns
 * from it.
 */
export async function loadWrapper(
  serviceName: string,
  actor: RawActor,
): Promise<{ wrapper: Wrapper; source: string; cleanup: () => void }> {
  const dir = mkdtempSync(join(tmpdir(), 'icp-bindgen-wrapper-'));
  const coreDir = join(dir, 'node_modules', '@icp-sdk', 'core');
  mkdirSync(coreDir, { recursive: true });
  writeFileSync(
    join(coreDir, 'package.json'),
    JSON.stringify({
      name: '@icp-sdk/core',
      type: 'module',
      exports: {
        './agent': './agent.js',
        './candid': './candid.js',
        './principal': './principal.js',
      },
    }),
  );
  for (const [file, source] of Object.entries(CORE_STUBS)) {
    writeFileSync(join(coreDir, file), source);
  }

  await generate({ didFile: `./tests/assets/${serviceName}.did`, outDir: dir });
  const wrapperPath = join(dir, `${serviceName}.ts`);
  const module = await import(wrapperPath);
  (globalThis as Record<string, unknown>)[ACTOR_KEY] = actor;

  return {
    wrapper: module.createActor('aaaaa-aa', { agent: {} }),
    source: readFileSync(wrapperPath, 'utf-8'),
    cleanup: () => rmSync(dir, { recursive: true, force: true }),
  };
}
