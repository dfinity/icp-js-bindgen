import { beforeAll, describe, expect, it } from 'vitest';
import { wasmGenerate } from '../src/core/generate/rs.ts';
import { testWasmInit } from './utils/wasm.ts';

const TESTS_ASSETS_DIR = './tests/assets';
const SNAPSHOTS_DIR = './snapshots/wasm-generate';

beforeAll(async () => {
  await testWasmInit();
});

describe('wasmGenerate', () => {
  it.each(['hello_world', 'example'])('should generate a bindgen', async (serviceName) => {
    const didFile = `${TESTS_ASSETS_DIR}/${serviceName}.did`;

    const result = wasmGenerate(didFile, serviceName);
    await expect(result.declarations_js).toMatchFileSnapshot(
      `${SNAPSHOTS_DIR}/${serviceName}/declarations/${serviceName}.did.js.snapshot`,
    );
    await expect(result.declarations_ts).toMatchFileSnapshot(
      `${SNAPSHOTS_DIR}/${serviceName}/declarations/${serviceName}.did.d.ts.snapshot`,
    );
    await expect(result.interface_ts).toMatchFileSnapshot(
      `${SNAPSHOTS_DIR}/${serviceName}/${serviceName}.d.ts.snapshot`,
    );
    await expect(result.service_ts).toMatchFileSnapshot(
      `${SNAPSHOTS_DIR}/${serviceName}/${serviceName}.ts.snapshot`,
    );
  });
});
