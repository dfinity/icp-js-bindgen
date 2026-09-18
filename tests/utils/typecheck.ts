import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import ts from 'typescript';

// Minimal type stubs for @icp-sdk/core so tsc can validate the generated .did.ts files.
export const AGENT_STUB = `
export type ActorMethod<Args extends unknown[], Return> = (...args: Args) => Promise<Return>;
export type ActorSubclass<T = unknown> = T;
export interface Agent { readonly _isAgent: true }
export interface HttpAgentOptions { host?: string }
export interface ActorConfig { canisterId: string; agent?: Agent }
export declare const Actor: { createActor<T>(factory: unknown, config: ActorConfig): ActorSubclass<T> };
export declare const HttpAgent: { createSync(options?: HttpAgentOptions): Agent };
`;

export const PRINCIPAL_STUB = `
export interface Principal { toText(): string; }
`;

export const CANDID_STUB = `
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

/**
 * A temporary project with the `@icp-sdk/core` stubs and a strict `tsconfig.json`, for
 * typechecking generated files with their `// @ts-nocheck` header stripped.
 */
export function setupTypecheckDir(): string {
  const tmpDir = mkdtempSync(join(tmpdir(), 'icp-bindgen-typecheck-'));
  const coreDir = join(tmpDir, 'node_modules', '@icp-sdk', 'core');
  mkdirSync(coreDir, { recursive: true });
  writeFileSync(join(coreDir, 'agent.d.ts'), AGENT_STUB);
  writeFileSync(join(coreDir, 'principal.d.ts'), PRINCIPAL_STUB);
  writeFileSync(join(coreDir, 'candid.d.ts'), CANDID_STUB);
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
  return tmpDir;
}

export function stripTsNocheck(source: string): string {
  return source
    .split('\n')
    .filter((line) => line.trim() !== '// @ts-nocheck')
    .join('\n');
}

/** Diagnostics `tsc` reports in exactly the given files, with the project's options. */
export function typecheckFiles(tmpDir: string, filePaths: string[]): readonly ts.Diagnostic[] {
  const { config } = ts.readConfigFile(join(tmpDir, 'tsconfig.json'), ts.sys.readFile);
  const parsed = ts.parseJsonConfigFileContent(config, ts.sys, tmpDir);
  const program = ts.createProgram(filePaths, parsed.options);
  return ts
    .getPreEmitDiagnostics(program)
    .filter((d) => d.file !== undefined && filePaths.includes(d.file.fileName));
}
