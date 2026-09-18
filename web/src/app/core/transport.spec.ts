import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { call, listen, openOutsideApp } from './transport';

/**
 * A fake `EventSource`, that counts its instances.
 *
 * It's the only way to prove what's subtle about this module: how many
 * connections it opens. jsdom doesn't provide one, and a real connection
 * would need a server.
 */
class FakeStream {
  static opened: FakeStream[] = [];
  static alive = 0;

  onmessage: ((message: { data: string }) => void) | null = null;
  closed = false;

  constructor(readonly url: string) {
    FakeStream.opened.push(this);
    FakeStream.alive += 1;
  }

  close() {
    this.closed = true;
    FakeStream.alive -= 1;
  }

  /** Pushes a message, the way the server would. */
  push(event: string, payload: unknown) {
    this.onmessage?.({ data: JSON.stringify({ event, payload }) });
  }
}

describe('transport', () => {
  /**
   * The unsubscribes of the running test.
   *
   * The stream is MODULE state — that's the whole point: one connection for
   * everyone. A test that left a subscriber behind would keep the stream
   * open for the next one, which would then count a connection it never
   * asked for. We return them all, every time.
   */
  let toLeave: (() => void)[] = [];

  /** Subscribes, and keeps the unsubscribe function. */
  async function subscribe(event: string, receive: (payload: unknown) => void) {
    const leave = await listen(event, receive);
    toLeave.push(leave);
    return leave;
  }

  beforeEach(() => {
    FakeStream.opened = [];
    FakeStream.alive = 0;
    toLeave = [];
    vi.stubGlobal('EventSource', FakeStream);
  });

  afterEach(() => {
    for (const leave of toLeave) {
      leave();
    }
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  /**
   * **The defect that motivated the sharing.**
   *
   * Each subscription used to open its own SSE connection, which never
   * closes on its own. The launcher opens three, and a browser allows only
   * six simultaneous connections per origin over HTTP/1.1: two tabs were
   * enough to consume them all, and the following requests waited for a
   * slot that never came.
   *
   * The symptom was silent — the requests weren't failing, they hadn't left
   * yet.
   */
  it('three subscriptions open ONLY ONE connection', async () => {
    await subscribe('a', () => {});
    await subscribe('b', () => {});
    await subscribe('c', () => {});

    expect(FakeStream.alive).toBe(1);
  });

  /** Each message is only delivered to those who asked for that name. */
  it('the stream is demultiplexed by name', async () => {
    const receivedA: unknown[] = [];
    const receivedB: unknown[] = [];
    await subscribe('a', (payload) => receivedA.push(payload));
    await subscribe('b', (payload) => receivedB.push(payload));

    FakeStream.opened[0].push('a', { value: 1 });
    FakeStream.opened[0].push('b', { value: 2 });
    FakeStream.opened[0].push('unknown', { value: 3 });

    expect(receivedA).toEqual([{ value: 1 }]);
    expect(receivedB).toEqual([{ value: 2 }]);
  });

  /** Two subscribers to the SAME name both get served. */
  it('several subscribers to the same event are all served', async () => {
    let first = 0;
    let second = 0;
    await subscribe('progress', () => (first += 1));
    await subscribe('progress', () => (second += 1));

    FakeStream.opened[0].push('progress', null);

    expect([first, second]).toEqual([1, 1]);
  });

  /**
   * The connection stays open as long as a subscriber remains, and closes
   * when the last one leaves: reopening it costs a round trip, keeping it
   * open for no one costs one of the browser's six slots.
   */
  it('the connection closes when the last subscriber leaves', async () => {
    const leaveA = await subscribe('a', () => {});
    const leaveB = await subscribe('b', () => {});

    leaveA();
    expect(FakeStream.alive).toBe(1);

    leaveB();
    expect(FakeStream.alive).toBe(0);
  });

  /** And it reopens cleanly on the next subscription. */
  it('a subscription after closing reopens the stream', async () => {
    const leave = await subscribe('a', () => {});
    leave();
    await subscribe('a', () => {});

    expect(FakeStream.alive).toBe(1);
    expect(FakeStream.opened.length).toBe(2);
  });

  /**
   * A message that can't be read doesn't break the stream: the next one
   * carries the full state, not a delta.
   */
  it('an unreadable message does not break the stream', async () => {
    const received: unknown[] = [];
    await subscribe('a', (payload) => received.push(payload));

    FakeStream.opened[0].onmessage?.({ data: 'not json' });
    FakeStream.opened[0].push('a', 'after');

    expect(received).toEqual(['after']);
  });

  /** An ordinary response returns its payload, as-is. */
  it('a command that succeeds returns what the server wrote', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: true, json: async () => ({ username: 'thesam1798' }) })),
    );

    await expect(call('status')).resolves.toEqual({ username: 'thesam1798' });
  });

  /**
   * **An error rejects with the error value as-is.**
   *
   * That's what `invoke` does: `errorMessage` therefore handles both
   * transports the same way, without knowing anything about them.
   */
  it('a command that fails rejects with the server value', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: false, json: async () => 'unknown command: thing' })),
    );

    await expect(call('thing')).rejects.toBe('unknown command: thing');
  });

  /** Outside the window, a URL opens through the browser itself. */
  it('outside Tauri, a URL opens in a new tab', async () => {
    const open = vi.fn();
    vi.stubGlobal('window', { ...globalThis.window, open });

    await openOutsideApp('https://microsoft.com/link');

    expect(open).toHaveBeenCalledWith('https://microsoft.com/link', '_blank', 'noopener');
  });
});
