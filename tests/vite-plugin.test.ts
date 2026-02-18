/**
 * Tests for the `icpBindgen` Vite plugin's `configureServer` hook, which
 * watches a `.did` file for changes and re-runs code generation.
 *
 * The `generate` function is mocked so these tests focus purely on watcher
 * lifecycle behavior: listener registration, cleanup on server restart, file
 * filtering, and the `disableWatch` opt-out.
 */

import { EventEmitter } from 'node:events';
import { resolve } from 'node:path';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { icpBindgen } from '../src/plugins/vite.ts';

vi.mock('../src/core/generate/index.ts', () => ({
  generate: vi.fn().mockResolvedValue(undefined),
}));

function createMockServer() {
  const watcher = new EventEmitter();
  return {
    watcher: Object.assign(watcher, {
      add: vi.fn(),
      off: watcher.removeListener.bind(watcher),
    }),
  };
}

const pluginOptions = {
  didFile: './test.did',
  outDir: './out',
};

// Our mock server only implements the subset of ViteDevServer used by the
// plugin (watcher.add/on/off), so a single cast is needed here.
function configureServer(
  plugin: ReturnType<typeof icpBindgen>,
  server: ReturnType<typeof createMockServer>,
) {
  // biome-ignore lint/suspicious/noExplicitAny: partial mock of ViteDevServer
  (plugin.configureServer as any)(server);
}

describe('icpBindgen vite plugin', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should register a change listener on configureServer', () => {
    const plugin = icpBindgen(pluginOptions);
    const server = createMockServer();

    configureServer(plugin, server);

    expect(server.watcher.listenerCount('change')).toBe(1);
  });

  it('should not register a listener when disableWatch is true', () => {
    const plugin = icpBindgen({ ...pluginOptions, disableWatch: true });
    const server = createMockServer();

    configureServer(plugin, server);

    expect(server.watcher.listenerCount('change')).toBe(0);
  });

  it('should clean up previous listener when configureServer is called again', () => {
    const plugin = icpBindgen(pluginOptions);
    const server = createMockServer();

    // Simulate multiple configureServer calls (e.g. Vite server restarts
    // reusing the same watcher).
    for (let i = 0; i < 10; i++) {
      configureServer(plugin, server);
    }

    expect(server.watcher.listenerCount('change')).toBe(1);
  });

  it('should trigger generate only for the watched did file', async () => {
    const { generate } = await import('../src/core/generate/index.ts');

    const plugin = icpBindgen(pluginOptions);
    const server = createMockServer();

    configureServer(plugin, server);

    // Emit a change for an unrelated file — should not trigger generate.
    server.watcher.emit('change', '/some/other/file.ts');
    await vi.waitFor(() => {});
    expect(generate).not.toHaveBeenCalled();

    // Emit a change for the watched did file — should trigger generate.
    server.watcher.emit('change', resolve(pluginOptions.didFile));
    await vi.waitFor(() => {
      expect(generate).toHaveBeenCalledOnce();
    });
  });
});
