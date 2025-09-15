import { tanstackViteConfig } from '@tanstack/config/vite';
import { defineConfig, mergeConfig } from 'vitest/config';
import packageJson from './package.json';

const config = defineConfig({
  test: {
    name: packageJson.name,
    dir: './tests',
    watch: false,
    typecheck: { enabled: true, tsconfig: './tsconfig.test.json' },
  },
  define: {
    __PKG_VERSION__: JSON.stringify(packageJson.version),
    __PKG_NAME__: JSON.stringify(packageJson.name),
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
    tsconfigPath: './tsconfig.json',
  }),
);
