import * as format from './format';

describe('bytes', () => {
  it('picks a unit that reads well', () => {
    expect(format.bytes(512)).toBe('512 B');
    expect(format.bytes(1_500)).toBe('2 KB');
    expect(format.bytes(1_500_000)).toBe('1.5 MB');
    expect(format.bytes(830_000_000)).toBe('830 MB');
    expect(format.bytes(2_400_000_000)).toBe('2.4 GB');
  });

  it('counts in multiples of a thousand, like the sources', () => {
    // A player compares our figure to what Modrinth or Mojang shows:
    // if they diverge by a factor of 1.024 they'll think it's a bug.
    expect(format.bytes(1_000)).toBe('1 KB');
    expect(format.bytes(1_000_000)).toBe('1.0 MB');
  });

  it('does not decorate small units', () => {
    // "1536.0 KB" doesn't read, and the decimal adds nothing below the
    // megabyte.
    expect(format.bytes(900)).toBe('900 B');
  });

  it('renders zero for what is not a usable number', () => {
    // A missing total arrives as zero; a failed calculation can give NaN.
    // Neither one should write "NaN B" in the window.
    expect(format.bytes(0)).toBe('0 B');
    expect(format.bytes(-5)).toBe('0 B');
    expect(format.bytes(Number.NaN)).toBe('0 B');
    expect(format.bytes(Number.POSITIVE_INFINITY)).toBe('0 B');
  });
});

describe('rate', () => {
  it('adds the second to the size', () => {
    expect(format.rate(8_200_000)).toBe('8.2 MB/s');
  });
});

describe('duration', () => {
  it('gives seconds under a minute', () => {
    expect(format.duration(0)).toBe('0 s');
    expect(format.duration(51)).toBe('51 s');
  });

  it('gives minutes and seconds beyond that', () => {
    // "3 min" would suggest a precision we don't have;
    // "3 min 12 s" reads like a countdown.
    expect(format.duration(192)).toBe('3 min 12 s');
    expect(format.duration(120)).toBe('2 min');
  });

  it('rounds to the minute beyond the hour', () => {
    expect(format.duration(3_600)).toBe('1 h');
    expect(format.duration(5_400)).toBe('1 h 30 min');
  });

  it('refuses to invent a duration that is not one', () => {
    expect(format.duration(-1)).toBe('—');
    expect(format.duration(Number.NaN)).toBe('—');
  });
});

describe('percentage', () => {
  it('relates what is acquired to the total', () => {
    expect(format.percentage(250, 1_000)).toBe(25);
  });

  it('never goes past a hundred', () => {
    // The total is a floor when a source doesn't publish its sizes: without
    // a bound, the bar would overflow its own width.
    expect(format.percentage(1_500, 1_000)).toBe(100);
  });

  it('renders zero when there is nothing to relate', () => {
    // Dividing by zero would give infinity, and a bar of width "Infinity%".
    expect(format.percentage(0, 0)).toBe(0);
    expect(format.percentage(500, 0)).toBe(0);
    expect(format.percentage(-10, 1_000)).toBe(0);
  });
});

describe('overallProgress', () => {
  it('counts the steps crossed, not the current batch', () => {
    // Without this, the bar goes back to zero at every step and "100%"
    // shows up seven times in a row — which makes it look like nothing is
    // happening.
    expect(format.overallProgress(0, 0, 9)).toBe(0);
    expect(format.overallProgress(0, 1, 9)).toBeCloseTo(11.11, 1);
    expect(format.overallProgress(8, 1, 9)).toBe(100);
  });

  it('bounds the fraction received', () => {
    // The total is a floor when a source doesn't publish its sizes: the
    // fraction can go past one.
    expect(format.overallProgress(1, 5, 9)).toBeCloseTo(22.22, 1);
    expect(format.overallProgress(1, -1, 9)).toBeCloseTo(11.11, 1);
  });

  it('never divides by zero', () => {
    expect(format.overallProgress(3, 0.5, 0)).toBe(0);
    expect(format.overallProgress(-1, 0.5, 9)).toBe(0);
  });
});

describe('hue', () => {
  it('gives the same color for the same account', () => {
    expect(format.hue('abcdef')).toBe(format.hue('abcdef'));
  });

  it('stays within the color wheel', () => {
    for (const uuid of ['a', 'cd7e6050f3ca4e4e886f44d93ac4bcc9', '']) {
      const value = format.hue(uuid);
      expect(value).toBeGreaterThanOrEqual(0);
      expect(value).toBeLessThan(360);
    }
  });
});

describe('initials', () => {
  it('takes the first two letters, capitalized', () => {
    expect(format.initials('thesam1798')).toBe('TH');
  });

  it('never renders nothing', () => {
    // An empty badge stands out more than a question mark.
    expect(format.initials('')).toBe('?');
    expect(format.initials('   ')).toBe('?');
  });
});

/**
 * **What moves the bar when nothing has weight.**
 *
 * Mod resolution queries the APIs one after another: some thirty seconds on
 * a fifty-mod pack, for a few kilobytes. And CurseForge doesn't publish
 * sizes without a key. The byte total is then zero, and the bar would stay
 * motionless — which nothing distinguishes from a crash.
 */
describe('batchFraction', () => {
  it('is based on weight when it is known', () => {
    expect(format.batchFraction(210, 840, 3, 4)).toBeCloseTo(0.25);
  });

  /** Weight wins: it's more faithful, files aren't all the same size. */
  it('weight wins over the count', () => {
    expect(format.batchFraction(0, 840, 51, 51)).toBe(0);
  });

  it('failing weight, it counts the settled requests', () => {
    expect(format.batchFraction(0, 0, 12, 51)).toBeCloseTo(12 / 51);
  });

  /** With nothing at all, zero — and above all no division by zero. */
  it('with nothing known, zero', () => {
    expect(format.batchFraction(0, 0, 0, 0)).toBe(0);
    expect(format.batchFraction(10, 0, 5, 0)).toBe(0);
  });
});
