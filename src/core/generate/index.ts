import { resolve, basename } from "node:path";
import { writeFile } from "node:fs/promises";
import { emptyDir, ensureDir } from "./fs.ts";
import { indexBinding, prepareBinding } from "./bindings.ts";
import { wasmGenerate, wasmInit, type WasmGenerateResult } from "./rs.ts";

const DID_FILE_EXTENSION = ".did";

type GenerateOptions = {
  didFile: string;
  outDir: string;
};

export async function generate(options: GenerateOptions) {
  await wasmInit();

  const { didFile, outDir } = options;
  const didFilePath = resolve(didFile);
  const outputFileName = basename(didFile, DID_FILE_EXTENSION);

  await emptyDir(outDir);
  await ensureDir(outDir);
  await ensureDir(resolve(outDir, "declarations"));

  const result = wasmGenerate(didFilePath, outputFileName);

  await writeBindings({
    bindings: result,
    outDir,
    outputFileName,
  });

  await writeIndex(outDir, outputFileName);
}

type WriteBindingsOptions = {
  bindings: WasmGenerateResult;
  outDir: string;
  outputFileName: string;
};

export async function writeBindings({
  bindings,
  outDir,
  outputFileName,
}: WriteBindingsOptions) {
  const declarationsTsFile = resolve(
    outDir,
    "declarations",
    `${outputFileName}.did.d.ts`
  );
  const declarationsJsFile = resolve(
    outDir,
    "declarations",
    `${outputFileName}.did.js`
  );
  const interfaceTsFile = resolve(outDir, `${outputFileName}.d.ts`);
  const serviceTsFile = resolve(outDir, `${outputFileName}.ts`);

  const declarationsTs = prepareBinding(bindings.declarations_ts);
  const declarationsJs = prepareBinding(bindings.declarations_js);
  const interfaceTs = prepareBinding(bindings.interface_ts);
  const serviceTs = prepareBinding(bindings.service_ts);

  await writeFile(declarationsTsFile, declarationsTs);
  await writeFile(declarationsJsFile, declarationsJs);
  await writeFile(interfaceTsFile, interfaceTs);
  await writeFile(serviceTsFile, serviceTs);
}

export async function writeIndex(outDir: string, outputFileName: string) {
  const indexFile = resolve(outDir, "index.ts");

  const index = indexBinding(outputFileName);
  await writeFile(indexFile, index);
}
