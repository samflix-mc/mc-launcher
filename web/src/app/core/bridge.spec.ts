import { describe, expect, it } from 'vitest';

import { errorMessage, playerHead } from './bridge';

/**
 * Two free functions, proven WITHOUT mounting a component.
 *
 * The old suite mounted the whole component to check these two functions.
 * Testing them here is a redistribution by subject, not a relocation: what
 * they do has nothing to do with a DOM.
 */

describe('errorMessage', () => {
  /**
   * The common case: `Error` serializes to a string on the Rust side, and
   * that's exactly what we want to display.
   */
  it('returns a string as-is', () => {
    expect(errorMessage('the session expired')).toBe('the session expired');
  });

  it('takes the message from an Error', () => {
    expect(errorMessage(new Error('network unreachable'))).toBe('network unreachable');
  });

  /**
   * `String({})` would give "[object Object]", which teaches nobody
   * anything — and it's exactly what a player would paste into a report.
   */
  it('never shows "[object Object]"', () => {
    expect(errorMessage({ code: 42 })).not.toContain('[object Object]');
    expect(errorMessage({ code: 42 })).toContain('42');
  });

  it("survives what doesn't serialize", () => {
    const cyclic: Record<string, unknown> = {};
    cyclic['self'] = cyclic;
    expect(() => errorMessage(cyclic)).not.toThrow();
  });
});

describe('playerHead', () => {
  it('builds the mc-heads address', () => {
    expect(playerHead('abc-123')).toBe('https://mc-heads.net/head/abc-123/96');
  });

  /**
   * The UUID comes from Microsoft, so from the outside. Without encoding,
   * an identifier containing a slash or a question mark would fall outside
   * the expected path.
   */
  it('encodes what comes from the outside', () => {
    expect(playerHead('a/b?c')).toBe('https://mc-heads.net/head/a%2Fb%3Fc/96');
  });

  it('accepts another size', () => {
    expect(playerHead('abc', 32)).toContain('/32');
  });
});
