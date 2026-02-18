import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { readFileSync, mkdirSync, writeFileSync, rmSync, readdirSync, statSync } from 'node:fs';
import { join, basename } from 'node:path';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import ts from 'typescript';
import { globSync } from 'tinyglobby';

// Minimal type stubs for @icp-sdk/core so tsc can validate the generated .did.ts files.
const AGENT_STUB = `
export type ActorMethod<Args extends unknown[], Return> = (...args: Args) => Promise<Return>;
`;

const PRINCIPAL_STUB = `
export interface Principal { toText(): string; }
`;

const CANDID_STUB = `
export namespace IDL {
  interface Type {}
  interface RecClass extends Type { fill(t: Type): void; }
  interface ServiceClass extends Type {}
  type InterfaceFactory = (args: { IDL: typeof IDL }) => ServiceClass;

  function Rec(): RecClass;
  function Service(methods: Record<string, Type>): ServiceClass;
  function Func(args: Type[], ret: Type[], modes: string[]): Type;
  function Record(fields: Record<string, Type>): Type;
  function Variant(fields: Record<string, Type>): Type;
  function Opt(t: Type): Type;
  function Vec(t: Type): Type;
  function Tuple(...ts: Type[]): Type;

  const Nat: Type;
  const Int: Type;
  const Text: Type;
  const Bool: Type;
  const Null: Type;
  const Principal: Type;
  const Nat8: Type;
  const Nat16: Type;
  const Nat32: Type;
  const Nat64: Type;
  const Int8: Type;
  const Int16: Type;
  const Int32: Type;
  const Int64: Type;
  const Float32: Type;
  const Float64: Type;
  const Empty: Type;
  const Reserved: Type;
}
`;

let tmpDir: string;

beforeAll(() => {
  tmpDir = mkdtempSync(join(tmpdir(), 'icp-bindgen-typecheck-'));

  // Write @icp-sdk/core stubs into a fake node_modules
  const coreDir = join(tmpDir, 'node_modules', '@icp-sdk', 'core');
  mkdirSync(coreDir, { recursive: true });
  writeFileSync(join(coreDir, 'agent.d.ts'), AGENT_STUB);
  writeFileSync(join(coreDir, 'principal.d.ts'), PRINCIPAL_STUB);
  writeFileSync(join(coreDir, 'candid.d.ts'), CANDID_STUB);

  // Write a tsconfig
  writeFileSync(
    join(tmpDir, 'tsconfig.json'),
    JSON.stringify({
      compilerOptions: {
        target: 'ES2023',
        module: 'ESNext',
        moduleResolution: 'bundler',
        strict: true,
        noEmit: true,
        skipLibCheck: true,
      },
      include: ['*.ts'],
    }),
  );
});

afterAll(() => {
  if (tmpDir) rmSync(tmpDir, { recursive: true, force: true });
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
