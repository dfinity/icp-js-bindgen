import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { testWasmInit } from './utils/wasm.ts';
import { loadWrapper, type RawActor } from './utils/wrapper.ts';

// Runs the generated wrapper against a stub actor and checks what reaches the wire.
// Snapshots and typechecks cannot catch conversion code that is valid but wrong.

// Every wrapper a test loads, removed after the test.
const cleanups: (() => void)[] = [];

beforeAll(async () => {
  await testWasmInit();
});

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

async function sentBy(serviceName: string, method: string, ...args: unknown[]) {
  const calls: unknown[][] = [];
  const actor: RawActor = new Proxy(
    {},
    {
      get:
        () =>
        async (...received: unknown[]) => {
          calls.push(received);
        },
    },
  );
  const loaded = await loadWrapper(serviceName, actor);
  cleanups.push(loaded.cleanup);
  await loaded.wrapper[method](...args);
  return calls[0];
}

async function receivedBy(serviceName: string, method: string, returned: unknown) {
  const actor: RawActor = new Proxy({}, { get: () => async () => returned });
  const loaded = await loadWrapper(serviceName, actor);
  cleanup = loaded.cleanup;
  return loaded.wrapper[method]();
}

describe('optional values', () => {
  it('sends present but falsy record fields as present', async () => {
    const [fields] = await sentBy('optional_presence', 'send_fields', {
      zero: 0n,
      no: false,
      empty: '',
    });

    expect(fields).toEqual({ zero: [0n], no: [false], empty: [''], missing: [] });
  });

  it('sends a present but falsy variant payload as present', async () => {
    const [payload] = await sentBy('optional_presence', 'send_payload', {
      __kind__: 'count',
      count: 0n,
    });

    expect(payload).toEqual({ count: [0n] });
  });

  it('sends a null variant payload as absent', async () => {
    const [payload] = await sentBy('optional_presence', 'send_payload', {
      __kind__: 'count',
      count: null,
    });

    expect(payload).toEqual({ count: [] });
  });

  it('sends a present but falsy optional argument as present', async () => {
    expect(await sentBy('optional_presence', 'send_opt', 0n)).toEqual([[0n]]);
    expect(await sentBy('optional_presence', 'send_opt', null)).toEqual([[]]);
  });
});

describe('nested optional record fields', () => {
  it('sends absent, present but empty, and set', async () => {
    const cfg = { url: 'https://example.org' };
    const [fields] = await sentBy('nested_option_fields', 'send_fields', {
      text_field: null,
      cfg_field: cfg,
    });

    expect(fields).toEqual({
      text_field: [[]],
      cfg_field: [[{ url: ['https://example.org'] }]],
      named_field: [],
    });
  });

  it('receives absent, present but empty, and set', async () => {
    const received = await receivedBy('nested_option_fields', 'get_fields', {
      text_field: [[]],
      cfg_field: [[{ url: ['https://example.org'] }]],
      named_field: [],
    });

    expect(received).toEqual({
      text_field: null,
      cfg_field: { url: 'https://example.org' },
      named_field: undefined,
    });
  });

  it('round-trips a field declared through a named option', async () => {
    const [fields] = await sentBy('nested_option_fields', 'send_fields', { named_field: null });
    expect(fields).toMatchObject({ named_field: [[]] });

    const received = await receivedBy('nested_option_fields', 'get_fields', {
      text_field: [],
      cfg_field: [],
      named_field: [['set']],
    });
    expect(received).toMatchObject({ named_field: 'set' });
  });
});
