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
    expect(fileExists(`${OUTPUT_DIR}/${serviceName}/${serviceName}.d.ts`)).toBe(false);
  });

  it.each(['hello_world', 'example'])(
    'should generate a bindgen with interface declaration',
    async (serviceName) => {
      const didFile = `${TESTS_ASSETS_DIR}/${serviceName}.did`;

      await generate({
        didFile,
        outDir: OUTPUT_DIR,
        output: { interfaceFile: true },
      });

      await expectGeneratedOutput(SNAPSHOTS_DIR, serviceName);

      const interfaceTs = await readFileFromOutput(`${serviceName}.d.ts`);
      await expect(interfaceTs).toMatchFileSnapshot(
        `${SNAPSHOTS_DIR}/${serviceName}/${serviceName}.d.ts.snapshot`,
      );
    },
  );

  it.each(['hello_world', 'example'])(
    'should generate a bindgen with declarations only',
    async (serviceName) => {
      const didFile = `${TESTS_ASSETS_DIR}/${serviceName}.did`;

      await generate({
        didFile,
        outDir: OUTPUT_DIR,
        output: { disableActor: true },
      });

      await expectGeneratedDeclarations(SNAPSHOTS_DIR, serviceName);
      expect(fileExists(`${OUTPUT_DIR}/${serviceName}/${serviceName}.d.ts`)).toBe(false);
      expect(fileExists(`${OUTPUT_DIR}/${serviceName}/${serviceName}.ts`)).toBe(false);
      expect(fileExists(`${OUTPUT_DIR}/${serviceName}/index.ts`)).toBe(false);
    },
  );

  it.each(['hello_world', 'example'])(
    'should throw when generating an interface file with declarations only',
    async (serviceName) => {
      const didFile = `${TESTS_ASSETS_DIR}/${serviceName}.did`;

      await expect(
        generate({
          didFile,
          outDir: OUTPUT_DIR,
          output: { disableActor: true, interfaceFile: true },
        }),
      ).rejects.toThrow('Cannot generate an interface file when generating the actor is disabled');

      expect(fileExists(`${OUTPUT_DIR}/${serviceName}/declarations/${serviceName}.did.d.ts`)).toBe(
        false,
      );
      expect(fileExists(`${OUTPUT_DIR}/${serviceName}/declarations/${serviceName}.did.js`)).toBe(
        false,
      );
    },
  );

  it('should preserve the .did file', async () => {
    const { readFile: realReadFile } =
      await vi.importActual<typeof import('node:fs/promises')>('node:fs/promises');

    const serviceName = 'hello_world';
    const didFile = `${TESTS_ASSETS_DIR}/${serviceName}.did`;
    const originalDidFileContent = await realReadFile(didFile, 'utf-8');

    // We simulate a .did file in the output directory
    const didFilePath = `${OUTPUT_DIR}/${serviceName}.did`;
    vol.mkdirSync(OUTPUT_DIR, { recursive: true });
    vol.writeFileSync(didFilePath, originalDidFileContent, { encoding: 'utf-8' });
    // We also add another (unused) .did file to check that all of them are preserved
    const otherDidFilePath = `${OUTPUT_DIR}/other.did`;
    vol.writeFileSync(otherDidFilePath, 'other', { encoding: 'utf-8' });

    // Assert before and after the generation
    expect(fileExists(didFilePath)).toBe(true);
    expect(fileExists(otherDidFilePath)).toBe(true);

    await generate({
      // We must use the file that exists in the real filesystem
      // because wasm reads from the real filesystem
      didFile,
      outDir: OUTPUT_DIR,
    });

    expect(fileExists(didFilePath)).toBe(true);
    expect(fileExists(otherDidFilePath)).toBe(true);
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

function fileExists(path: string): boolean {
  return vol.existsSync(path);
}

async function expectGeneratedDeclarations(
  snapshotsDir: string,
  serviceName: string,
): Promise<void> {
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
}

async function expectGeneratedOutput(snapshotsDir: string, serviceName: string): Promise<void> {
  const generatedOutputDir = join(snapshotsDir, serviceName);

  await expectGeneratedDeclarations(snapshotsDir, serviceName);

  const serviceTs = await readFileFromOutput(`${serviceName}.ts`);
  await expect(serviceTs).toMatchFileSnapshot(`${generatedOutputDir}/${serviceName}.ts.snapshot`);

  const indexTs = await readFileFromOutput('index.ts');
  await expect(indexTs).toMatchFileSnapshot(`${generatedOutputDir}/index.ts.snapshot`);
}
