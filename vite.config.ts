import { defineConfig, mergeConfig } from "vitest/config";
import { tanstackViteConfig } from "@tanstack/config/vite";
import { viteStaticCopy } from "vite-plugin-static-copy";
import packageJson from "./package.json";

const CORE_RS_DIR = "core/generate/rs";
const CORE_RS_DIST_WASM_FILE = `./src/${CORE_RS_DIR}/dist/icp-js-bindgen_bg.wasm`;
const CORE_RS_DIST_WASM_DECLARATIONS_FILE = `./src/${CORE_RS_DIR}/dist/icp-js-bindgen_bg.wasm.d.ts`;

const config = defineConfig({
  test: {
    name: packageJson.name,
    dir: "./tests",
    watch: false,
    typecheck: { enabled: true, tsconfig: "./tsconfig.test.json" },
  },
  define: {
    __PKG_VERSION__: JSON.stringify(packageJson.version),
  },
  plugins: [
    viteStaticCopy({
      targets: [
        {
          src: CORE_RS_DIST_WASM_FILE,
          dest: `esm/${CORE_RS_DIR}/dist`,
        },
        {
          src: CORE_RS_DIST_WASM_DECLARATIONS_FILE,
          dest: `esm/${CORE_RS_DIR}/dist`,
        },
        {
          src: CORE_RS_DIST_WASM_FILE,
          dest: `cjs/${CORE_RS_DIR}/dist`,
        },
        {
          src: CORE_RS_DIST_WASM_DECLARATIONS_FILE,
          dest: `cjs/${CORE_RS_DIR}/dist`,
        },
      ],
    }),
  ],
});

export default mergeConfig(
  config,
  tanstackViteConfig({
    entry: [
      "./src/index.ts",
      "./src/core/index.ts",
      "./src/plugins/vite.ts",
      "./src/cli/icp-bindgen.ts",
    ],
    srcDir: "./src",
    outDir: "./dist",
    tsconfigPath: "./tsconfig.json",
  })
);
