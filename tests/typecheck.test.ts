import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { globSync } from 'tinyglobby';
import ts from 'typescript';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setupTypecheckDir } from './utils/typecheck.ts';

let tmpDir: string;

beforeAll(() => {
  tmpDir = setupTypecheckDir();
});

afterAll(() => {
  if (tmpDir) rmSync(tmpDir, { recursive: true, force: true });
});

describe('typecheck .did.ts with allowJs false', () => {
  const snapshotFiles = globSync('tests/snapshots/**/*.did.ts.snapshot');

  it.each(snapshotFiles)('%s', (snapshotPath) => {
    const content = readFileSync(snapshotPath, 'utf-8');
    const tsFileName = basename(snapshotPath).replace('.snapshot', '');
    const tsFilePath = join(tmpDir, tsFileName);

    writeFileSync(tsFilePath, content);

    // A consumer file that imports the generated .did.ts module.
    const consumerPath = join(tmpDir, `consumer_${tsFileName}`);
    const importBase = tsFileName.replace(/\.ts$/, '');
    writeFileSync(
      consumerPath,
      `import { idlFactory } from './${importBase}';\nconsole.log(idlFactory);\n`,
    );

    const configPath = join(tmpDir, 'tsconfig.json');
    const rawConfig = JSON.parse(readFileSync(configPath, 'utf-8'));
    rawConfig.compilerOptions.allowJs = false;
    const { config } = ts.readConfigFile(configPath, () => JSON.stringify(rawConfig));
    const parsed = ts.parseJsonConfigFileContent(config, ts.sys, tmpDir);

    const program = ts.createProgram([consumerPath], parsed.options);
    const diagnostics = ts
      .getPreEmitDiagnostics(program)
      .filter((d) => d.file?.fileName === consumerPath || d.file?.fileName === tsFilePath);

    if (diagnostics.length > 0) {
      const formatted = ts.formatDiagnosticsWithColorAndContext(diagnostics, {
        getCanonicalFileName: (f) => f,
        getCurrentDirectory: () => tmpDir,
        getNewLine: () => '\n',
      });
      expect.fail(`TypeScript errors in ${snapshotPath}:\n${formatted}`);
    }
  });
});

describe('typecheck .did.ts snapshots', () => {
  const snapshotFiles = globSync('tests/snapshots/**/*.did.ts.snapshot');

  it.each(snapshotFiles)('%s', (snapshotPath) => {
    const content = readFileSync(snapshotPath, 'utf-8');
    const tsFileName = basename(snapshotPath).replace('.snapshot', '');
    const tsFilePath = join(tmpDir, tsFileName);

    writeFileSync(tsFilePath, content);

    const configPath = join(tmpDir, 'tsconfig.json');
    const { config } = ts.readConfigFile(configPath, ts.sys.readFile);
    const parsed = ts.parseJsonConfigFileContent(config, ts.sys, tmpDir);

    const program = ts.createProgram([tsFilePath], parsed.options);
    const diagnostics = ts
      .getPreEmitDiagnostics(program)
      .filter((d) => d.file?.fileName === tsFilePath);

    if (diagnostics.length > 0) {
      const formatted = ts.formatDiagnosticsWithColorAndContext(diagnostics, {
        getCanonicalFileName: (f) => f,
        getCurrentDirectory: () => tmpDir,
        getNewLine: () => '\n',
      });
      expect.fail(`TypeScript errors in ${snapshotPath}:\n${formatted}`);
    }
  });
});

/**
 * Typecheck the generated *wrapper* (`<service>.ts`), not just the declarations.
 *
 * The wrapper ships with a `// @ts-nocheck` header, so type errors in it are invisible to
 * consumers and to `tsc`. That is why https://github.com/dfinity/icp-js-bindgen/issues/151
 * (enum member references that did not resolve) shipped unnoticed. The header is stripped
 * here so CI sees what TypeScript would otherwise report.
 */
describe('typecheck generated wrapper', () => {
  const wrapperSnapshots = globSync('tests/snapshots/generate/*/*.ts.snapshot').filter(
    (p) => !p.endsWith('.d.ts.snapshot'),
  );

  it('finds wrapper snapshots to check', () => {
    expect(wrapperSnapshots.length).toBeGreaterThan(0);
  });

  it.each(wrapperSnapshots)('%s', (snapshotPath) => {
    const serviceName = basename(snapshotPath, '.ts.snapshot');

    // The wrapper imports `./declarations/<service>.did` extensionless. Depending on the
    // options a fixture was generated with, that resolves to either a single `.did.ts` or
    // to `.did.js` + `.did.d.ts`; for typechecking, the `.did.d.ts` stands in for the pair.
    const declarationsDir = join(dirname(snapshotPath), 'declarations');
    const [declarationsExt, declarationsSnapshot] = existsSync(
      join(declarationsDir, `${serviceName}.did.ts.snapshot`),
    )
      ? ['.did.ts', join(declarationsDir, `${serviceName}.did.ts.snapshot`)]
      : ['.did.d.ts', join(declarationsDir, `${serviceName}.did.d.ts.snapshot`)];

    // Each wrapper gets its own directory so same-named files cannot collide. node_modules
    // resolution walks up to the stubs written at tmpDir.
    const caseDir = join(tmpDir, `wrapper_${serviceName}`);
    mkdirSync(join(caseDir, 'declarations'), { recursive: true });

    const wrapper = readFileSync(snapshotPath, 'utf-8')
      .split('\n')
      .filter((line) => line.trim() !== '// @ts-nocheck')
      .join('\n');
    const wrapperPath = join(caseDir, `${serviceName}.ts`);
    writeFileSync(wrapperPath, wrapper);

    const declarationsPath = join(caseDir, 'declarations', `${serviceName}${declarationsExt}`);
    writeFileSync(declarationsPath, readFileSync(declarationsSnapshot, 'utf-8'));

    const configPath = join(tmpDir, 'tsconfig.json');
    const { config } = ts.readConfigFile(configPath, ts.sys.readFile);
    const parsed = ts.parseJsonConfigFileContent(config, ts.sys, tmpDir);

    const program = ts.createProgram([wrapperPath, declarationsPath], parsed.options);
    const diagnostics = ts
      .getPreEmitDiagnostics(program)
      .filter((d) => d.file?.fileName === wrapperPath || d.file?.fileName === declarationsPath);

    if (diagnostics.length > 0) {
      const formatted = ts.formatDiagnosticsWithColorAndContext(diagnostics, {
        getCanonicalFileName: (f) => f,
        getCurrentDirectory: () => tmpDir,
        getNewLine: () => '\n',
      });
      expect.fail(`TypeScript errors in ${snapshotPath}:\n${formatted}`);
    }
  });
});
