import { describe, expect, it } from 'vitest';

import { sayTheDate } from './dates';

/**
 * The reference instant is PASSED AS A PARAMETER, and that's what makes
 * these tests possible. Without it, we'd have to fake the clock, and the
 * results would depend on the day the suite runs.
 */
const NOW = new Date('2026-09-18T12:00:00Z');

describe('sayTheDate', () => {
  it('says "just now" for what has just happened', () => {
    expect(sayTheDate('2026-09-18T11:59:30Z', NOW)).toBe('just now');
  });

  it('counts in minutes, then hours, then days', () => {
    expect(sayTheDate('2026-09-18T11:30:00Z', NOW)).toContain('30');
    expect(sayTheDate('2026-09-18T09:00:00Z', NOW)).toContain('3');
  });

  /**
   * `numeric: 'auto'` renders "yesterday" rather than "1 day ago", and
   * that's what we want: a language names the nearest day rather than
   * counting it.
   */
  it('says "yesterday" rather than a count', () => {
    expect(sayTheDate('2026-09-17T12:00:00Z', NOW)).toBe('yesterday');
  });

  /**
   * Beyond that, English has no distinct word for two days back the way
   * French does — it counts, same as for any greater distance.
   */
  it('counts the days beyond yesterday', () => {
    expect(sayTheDate('2026-09-16T12:00:00Z', NOW)).toContain('2');
    expect(sayTheDate('2026-09-14T12:00:00Z', NOW)).toContain('4');
  });

  /**
   * Beyond a week, a date rather than a count.
   *
   * "47 days ago" asks for mental arithmetic that "August 2" doesn't:
   * that's roughly the horizon where people stop counting.
   */
  it('switches to a date beyond a week', () => {
    const said = sayTheDate('2026-08-02T12:00:00Z', NOW);
    expect(said).toContain('August');
    // Same year: the year isn't repeated, it wouldn't teach anything.
    expect(said).not.toContain('2026');
  });

  it('adds the year when it is not the current one', () => {
    expect(sayTheDate('2024-08-02T12:00:00Z', NOW)).toContain('2024');
  });

  /**
   * Rust already validates the format, but a feed served by a modified host
   * could pass something else. Showing "Invalid Date" to a player would be
   * worse than showing nothing at all.
   */
  it('says nothing rather than "Invalid Date"', () => {
    expect(sayTheDate('not a date', NOW)).toBe('');
    expect(sayTheDate('', NOW)).toBe('');
  });
});
