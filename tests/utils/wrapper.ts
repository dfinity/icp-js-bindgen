import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { generate } from '../../src/core/generate/index.ts';

// Just enough of @icp-sdk/core for the generated modules to load: the wrapper class only
// forwards to the actor it is constructed with, so no agent or IDL value is ever used.
const CORE_STUBS = {
  'agent.js': 'export const Actor = {};\nexport const HttpAgent = {};\n',
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
 * Generates the bindings for `tests/assets/<serviceName>.did` and wraps `actor` in the
 * generated class, so a test sees what the wrapper sends to and returns from the actor.
 */
export async function loadWrapper(
  serviceName: string,
  actor: RawActor,
): Promise<{ wrapper: Wrapper; cleanup: () => void }> {
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
  const module = await import(join(dir, `${serviceName}.ts`));
  const className = serviceName.charAt(0).toUpperCase() + serviceName.slice(1);

  return {
    wrapper: new module[className](actor),
    cleanup: () => rmSync(dir, { recursive: true, force: true }),
  };
}
