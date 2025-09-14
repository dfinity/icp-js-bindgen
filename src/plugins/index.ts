export interface Options {
  /**
   * The path to the `.did` file.
   */
  didFile: string;
  /**
   * The path to the directory where the bindings will be generated.
   */
  outDir: string;
  /**
   * Disables watching for changes in the `.did` file when using the dev server.
   *
   * @default false
   */
  disableWatch?: boolean;
  /**
   * Additional features to generate bindings with.
   */
  additionalFeatures?: {
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
}
