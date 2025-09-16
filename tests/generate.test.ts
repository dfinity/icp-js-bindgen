// biome-ignore assist/source/organizeImports: vi must be imported before memfs and memfs must be imported before readFile
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fs as memFs, vol } from 'memfs';
import { readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { generate } from '../src/core/generate/index.ts';
import { testWasmInit } from './utils/wasm.ts';

vi.mock('node:fs/promises', () => ({
  default: memFs.promises,
  ...memFs.promises,
}));

const TESTS_ASSETS_DIR = './tests/assets';
const OUTPUT_DIR = 'output';
const SNAPSHOTS_DIR = './assets/snapshots/generate';

beforeAll(async () => {
  await testWasmInit();
});

beforeEach(() => {
  vol.reset();
});

describe('generate', () => {
  it.each(['hello_world', 'example'])('should generate a bindgen', async (serviceName) => {
    const didFile = `${TESTS_ASSETS_DIR}/${serviceName}.did`;

    await generate({ didFile, outDir: OUTPUT_DIR });
    await expectGeneratedOutput(SNAPSHOTS_DIR, serviceName);
  });

  it('should generate a bindgen with canister env feature', async () => {
    const helloWorldServiceName = 'hello_world';
    const helloWorldDidFile = `${TESTS_ASSETS_DIR}/${helloWorldServiceName}.did`;

    await generate({
      didFile: helloWorldDidFile,
      outDir: OUTPUT_DIR,
      additionalFeatures: {
        canisterEnv: {
          variableNames: ['IC_CANISTER_ID:backend'],
        },
      },
    });

    await expectGeneratedOutput(SNAPSHOTS_DIR, helloWorldServiceName);
    await expect(await readFileFromOutput('canister-env.d.ts')).toMatchFileSnapshot(
      `${SNAPSHOTS_DIR}/${helloWorldServiceName}/canister-env.d.ts.snapshot`,
    );
  });
});

async function readFileFromOutput(path: string): Promise<string> {
  return await readFile(resolve(OUTPUT_DIR, path), 'utf-8');
}

async function expectGeneratedOutput(snapshotsDir: string, serviceName: string): Promise<void> {
  const generatedOutputDir = join(snapshotsDir, serviceName);
  const generatedOutputDeclarationsDir = join(generatedOutputDir, 'declarations');

  const declarationsJs = await readFileFromOutput(`declarations/${serviceName}.did.js`);
  await expect(declarationsJs).toMatchFileSnapshot(
    `${generatedOutputDeclarationsDir}/${serviceName}.did.js.snapshot`,
  );

  const declarationsTs = await readFileFromOutput(`declarations/${serviceName}.did.d.ts`);
  await expect(declarationsTs).toMatchFileSnapshot(
    `${generatedOutputDeclarationsDir}/${serviceName}.did.d.ts.snapshot`,
  );

  const interfaceTs = await readFileFromOutput(`${serviceName}.d.ts`);
  await expect(interfaceTs).toMatchFileSnapshot(
    `${generatedOutputDir}/${serviceName}.d.ts.snapshot`,
  );

  const serviceTs = await readFileFromOutput(`${serviceName}.ts`);
  await expect(serviceTs).toMatchFileSnapshot(`${generatedOutputDir}/${serviceName}.ts.snapshot`);

  const indexTs = await readFileFromOutput('index.ts');
  await expect(indexTs).toMatchFileSnapshot(`${generatedOutputDir}/index.ts.snapshot`);
}
