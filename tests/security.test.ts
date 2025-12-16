import { beforeAll, describe, expect, it } from 'vitest';
import { wasmGenerate } from '../src/core/generate/rs.ts';
import { testWasmInit } from './utils/wasm.ts';

const TESTS_ASSETS_DIR = './tests/assets';
const SNAPSHOTS_BASE_DIR = './snapshots/wasm-generate';
const SERVICE_NAME = 'malicious_doc';

beforeAll(async () => {
  await testWasmInit();
});

describe('security', () => {
  describe('doc comment injection prevention', () => {
    it('should escape */ in doc comments to prevent code injection', async () => {
      const didFile = `${TESTS_ASSETS_DIR}/${SERVICE_NAME}.did`;

      const result = wasmGenerate({
        did_file_path: didFile,
        service_name: SERVICE_NAME,
        declarations: {
          root_exports: true,
        },
      });

      await expect(result.declarations_js).toMatchFileSnapshot(
        `${SNAPSHOTS_BASE_DIR}/${SERVICE_NAME}/declarations/${SERVICE_NAME}.did.js.snapshot`,
      );
      await expect(result.declarations_ts).toMatchFileSnapshot(
        `${SNAPSHOTS_BASE_DIR}/${SERVICE_NAME}/declarations/${SERVICE_NAME}.did.d.ts.snapshot`,
      );
      await expect(result.interface_ts).toMatchFileSnapshot(
        `${SNAPSHOTS_BASE_DIR}/${SERVICE_NAME}/${SERVICE_NAME}.d.ts.snapshot`,
      );
      await expect(result.service_ts).toMatchFileSnapshot(
        `${SNAPSHOTS_BASE_DIR}/${SERVICE_NAME}/${SERVICE_NAME}.ts.snapshot`,
      );
    });
  });
});
