import { tanstackViteConfig } from '@tanstack/config/vite';
import { defineConfig, mergeConfig } from 'vite';
import packageJson from './package.json';

const config = defineConfig({
  define: {
    __PKG_VERSION__: JSON.stringify(packageJson.version),
    __PKG_NAME__: JSON.stringify(packageJson.name),
    __BIN_NAME__: JSON.stringify(Object.keys(packageJson.bin)[0]),
  },
});

export default mergeConfig(
  config,
  tanstackViteConfig({
    entry: [
      './src/index.ts',
      './src/core/index.ts',
      './src/plugins/vite.ts',
      './src/cli/icp-bindgen.ts',
    ],
    srcDir: './src',
    outDir: './dist',
    tsconfigPath: './tsconfig.lib.json',
  }),
);
