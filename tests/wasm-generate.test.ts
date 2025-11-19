import { beforeAll, describe, expect, it } from 'vitest';
import { wasmGenerate } from '../src/core/generate/rs.ts';
import { testWasmInit } from './utils/wasm.ts';

const TESTS_ASSETS_DIR = './tests/assets';
const SNAPSHOTS_BASE_DIR = './snapshots/wasm-generate';

beforeAll(async () => {
  await testWasmInit();
});

describe('wasmGenerate', () => {
  describe('with rootExport: false', () => {
    it.each(['hello_world', 'example'])('should generate a bindgen for %s', async (serviceName) => {
      const didFile = `${TESTS_ASSETS_DIR}/${serviceName}.did`;
      const snapshotsDir = `${SNAPSHOTS_BASE_DIR}/no-root-export`;

      const result = wasmGenerate({
        did_file: { LocalPath: didFile },
        service_name: serviceName,
        declarations: {
          root_exports: false,
        },
      });
      await expect(result.declarations_js).toMatchFileSnapshot(
        `${snapshotsDir}/${serviceName}/declarations/${serviceName}.did.js.snapshot`,
      );
      await expect(result.declarations_ts).toMatchFileSnapshot(
        `${snapshotsDir}/${serviceName}/declarations/${serviceName}.did.d.ts.snapshot`,
      );
      await expect(result.interface_ts).toMatchFileSnapshot(
        `${snapshotsDir}/${serviceName}/${serviceName}.d.ts.snapshot`,
      );
      await expect(result.service_ts).toMatchFileSnapshot(
        `${snapshotsDir}/${serviceName}/${serviceName}.ts.snapshot`,
      );
    });
  });

  describe('with rootExport: true', () => {
    it.each(['hello_world', 'example'])('should generate a bindgen for %s', async (serviceName) => {
      const didFile = `${TESTS_ASSETS_DIR}/${serviceName}.did`;
      const snapshotsDir = `${SNAPSHOTS_BASE_DIR}/root-export`;

      const result = wasmGenerate({
        did_file: { LocalPath: didFile },
        service_name: serviceName,
        declarations: {
          root_exports: true,
        },
      });
      await expect(result.declarations_js).toMatchFileSnapshot(
        `${snapshotsDir}/${serviceName}/declarations/${serviceName}.did.js.snapshot`,
      );
      await expect(result.declarations_ts).toMatchFileSnapshot(
        `${snapshotsDir}/${serviceName}/declarations/${serviceName}.did.d.ts.snapshot`,
      );
      await expect(result.interface_ts).toMatchFileSnapshot(
        `${snapshotsDir}/${serviceName}/${serviceName}.d.ts.snapshot`,
      );
      await expect(result.service_ts).toMatchFileSnapshot(
        `${snapshotsDir}/${serviceName}/${serviceName}.ts.snapshot`,
      );
    });
  });
});
