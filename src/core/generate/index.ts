import { resolve, basename } from "node:path";
import { writeFile } from "node:fs/promises";
import { emptyDir, ensureDir } from "./fs.ts";
import { indexBinding, prepareBinding } from "./bindings.ts";
import { wasmGenerate, wasmInit, type WasmGenerateResult } from "./rs.ts";

const DID_FILE_EXTENSION = ".did";

type GenerateOptions = {
  didFile: string;
  bindingsOutDir: string;
};

export async function generate(options: GenerateOptions) {
  await wasmInit();

  const { didFile, bindingsOutDir } = options;
  const didFilePath = resolve(didFile);
  const outputFileName = basename(didFile, DID_FILE_EXTENSION);

  await emptyDir(bindingsOutDir);
  await ensureDir(bindingsOutDir);
  await ensureDir(resolve(bindingsOutDir, "declarations"));

  const result = wasmGenerate(didFilePath, outputFileName);

  await writeBindings({
    bindings: result,
    bindingsOutDir,
    outputFileName,
  });

  await writeIndex(bindingsOutDir, outputFileName);
}

type WriteBindingsOptions = {
  bindings: WasmGenerateResult;
  bindingsOutDir: string;
  outputFileName: string;
};

export async function writeBindings({
  bindings,
  bindingsOutDir,
  outputFileName,
}: WriteBindingsOptions) {
  const declarationsTsFile = resolve(
    bindingsOutDir,
    "declarations",
    `${outputFileName}.did.d.ts`
  );
  const declarationsJsFile = resolve(
    bindingsOutDir,
    "declarations",
    `${outputFileName}.did.js`
  );
  const interfaceTsFile = resolve(bindingsOutDir, `${outputFileName}.d.ts`);
  const serviceTsFile = resolve(bindingsOutDir, `${outputFileName}.ts`);

  const declarationsTs = prepareBinding(bindings.declarations_ts);
  const declarationsJs = prepareBinding(bindings.declarations_js);
  const interfaceTs = prepareBinding(bindings.interface_ts);
  const serviceTs = prepareBinding(bindings.service_ts);

  await writeFile(declarationsTsFile, declarationsTs);
  await writeFile(declarationsJsFile, declarationsJs);
  await writeFile(interfaceTsFile, interfaceTs);
  await writeFile(serviceTsFile, serviceTs);
}

export async function writeIndex(
  bindingsOutDir: string,
  outputFileName: string
) {
  const indexFile = resolve(bindingsOutDir, "index.ts");

  const index = indexBinding(outputFileName);
  await writeFile(indexFile, index);
}
