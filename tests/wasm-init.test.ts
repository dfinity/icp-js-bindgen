import { describe, expect, it } from 'vitest';
import { getWasm } from './utils/wasm.ts';

describe('wasmInit', () => {
  it('should handle concurrent calls without hanging', async () => {
    const { wasmInit } = await import('../src/core/generate/rs.ts');
    const { wasm } = await getWasm();

    // Simulate multiple concurrent wasmInit calls, as would happen when
    // multiple icpBindgen() plugin instances trigger buildStart in parallel.
    const results = await Promise.all([
      wasmInit({ module_or_path: wasm }),
      wasmInit({ module_or_path: wasm }),
      wasmInit({ module_or_path: wasm }),
      wasmInit({ module_or_path: wasm }),
      wasmInit({ module_or_path: wasm }),
    ]);

    // All calls should resolve (not hang) and return undefined.
    for (const result of results) {
      expect(result).toBeUndefined();
    }
  });

  it('should handle concurrent calls without explicit args without hanging', async () => {
    const { wasmInit } = await import('../src/core/generate/rs.ts');
    const results = await Promise.all([wasmInit(), wasmInit(), wasmInit(), wasmInit(), wasmInit()]);

    // All calls should resolve (not hang) and return undefined.
    for (const result of results) {
      expect(result).toBeUndefined();
    }
  });
});
