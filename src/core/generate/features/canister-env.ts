import { resolve } from 'node:path';
import { DISCLAIMER_COMMENT, ESLINT_DISABLE_COMMENT } from '../constants';
import { writeFile } from 'node:fs/promises';

const OUTPUT_CANISTER_ENV_FILE = 'canister-env.d.ts';

function envBinding(varNames: string[]): string {
  if (varNames.length === 0) {
    return '';
  }

  // we don't want to disable typescript checks for this file
  let env = `${ESLINT_DISABLE_COMMENT}\n\n${DISCLAIMER_COMMENT}\n\ninterface CanisterEnv {\n`;
  for (const varName of varNames) {
    env += `  readonly ["${varName}"]: string;\n`;
  }
  env += '}';
  return env;
}

export async function generateCanisterEnv(varNames: string[], outDir: string) {
  const canisterEnv = envBinding(varNames);
  await writeFile(resolve(outDir, OUTPUT_CANISTER_ENV_FILE), canisterEnv);
}
