import { execSync } from 'node:child_process';
import { beforeAll, describe, expect, it } from 'vitest';
import packageJson from '../package.json';
import { testWasmInit } from './utils/wasm.ts';

const BIN_FILE = packageJson.bin['icp-bindgen'];

const SNAPSHOTS_DIR = './snapshots/cli';

beforeAll(async () => {
  await testWasmInit();
});

describe('cli', () => {
  it('should return help message', async () => {
    const result = execBin(['--help']);

    await expect(result).toMatchFileSnapshot(`${SNAPSHOTS_DIR}/help.snapshot`);
  });
});

function execBin(args: string[]): string {
  const output = execSync(`node ${BIN_FILE} ${args.join(' ')}`);
  return output.toString();
}
