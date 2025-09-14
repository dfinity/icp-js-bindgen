import { generateCanisterEnv } from './canister-env.ts';

export type GenerateAdditionalFeaturesOptions = {
  /**
   * If defined, generates a `canister-env.d.ts` file according to the provided options.
   */
  canisterEnv?: {
    /**
     * The variable names to include in the `canister-env.d.ts` file.
     */
    variableNames: string[];
  };
};

export async function generateAdditionalFeatures(
  options: GenerateAdditionalFeaturesOptions,
  outDir: string,
) {
  if (options.canisterEnv) {
    await generateCanisterEnv(options.canisterEnv.variableNames, outDir);
  }
}
