/**
 * Put numbers in a shape you can read at a glance.
 *
 * Nothing here is specific to the launcher, and nothing calls Rust: these are
 * pure functions, precisely so they can be verified without a window.
 */

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const;

/**
 * A size in bytes, in whichever unit makes it readable.
 *
 * Multiples of 1000, not 1024: that's what the sites the files come from
 * announce, and a player comparing our "830 MB" to their "830 MB" should
 * find the same number.
 *
 * One decimal from the megabyte up, none below: "1.5 MB" reads, "1536.0 KB"
 * doesn't.
 */
export function bytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return '0 B';
  }

  let remainder = value;
  let rank = 0;
  while (remainder >= 1000 && rank < UNITS.length - 1) {
    remainder /= 1000;
    rank += 1;
  }

  const decimals = rank >= 2 && remainder < 100 ? 1 : 0;
  return `${remainder.toFixed(decimals)} ${UNITS[rank]}`;
}

/** A rate, per second. */
export function rate(bytesPerSecond: number): string {
  return `${bytes(bytesPerSecond)}/s`;
}

/**
 * A duration, with no unit that doesn't earn its place.
 *
 * Past a minute, seconds are given too: "3 min" would suggest a precision we
 * don't have, and "3 min 12 s" reads like a countdown. Past the hour, we
 * round — at that point the nearest minute adds nothing.
 */
export function duration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) {
    return '—';
  }

  const whole = Math.round(seconds);
  if (whole < 60) {
    return `${whole} s`;
  }
  if (whole < 3600) {
    const minutes = Math.floor(whole / 60);
    const remainder = whole % 60;
    return remainder === 0 ? `${minutes} min` : `${minutes} min ${remainder} s`;
  }

  const hours = Math.floor(whole / 3600);
  const minutes = Math.round((whole % 3600) / 60);
  return minutes === 0 ? `${hours} h` : `${hours} h ${minutes} min`;
}

/**
 * A fraction as a percentage, bounded.
 *
 * The announced total is a floor when a source doesn't publish its sizes:
 * without a bound, the bar would go past its own width.
 */
export function percentage(acquired: number, total: number): number {
  if (!(total > 0)) {
    return 0;
  }
  return Math.max(0, Math.min(100, (acquired / total) * 100));
}

/**
 * How far along the current step is, between zero and one.
 *
 * **Two measures, and the second one isn't a fallback of last resort.** Weight
 * is used when known: it's the most faithful, since files aren't all the same
 * size. But mod resolution barely downloads anything — some thirty seconds of
 * API polling for a few kilobytes — and CurseForge doesn't publish sizes
 * without a key: the total byte count is then zero, and the fraction would
 * stay at zero the whole time.
 *
 * The count of settled requests, on the other hand, moves. A fifty-mod pack
 * therefore shows a bar that moves fifty times rather than a motionless bar —
 * which nothing distinguishes from a crash, and that's exactly the complaint
 * made about the install screen.
 */
export function batchFraction(
  bytesDone: number,
  total: number,
  files: number,
  filesTotal: number,
): number {
  if (total > 0) {
    return bytesDone / total;
  }
  if (filesTotal > 0) {
    return files / filesTotal;
  }
  return 0;
}

/**
 * The progress of the whole install, not just the current batch.
 *
 * Each step announces its own batch, so the batch bar goes back to zero seven
 * times. Seeing "100%" seven times in a row is exactly what makes it look
 * like nothing is happening: we count the steps crossed instead, and the
 * batch fraction only advances within its own step.
 *
 * `steps` is the number of steps on the path that actually count — the nine
 * of the install, not "ready" or "launched", which aren't work.
 */
export function overallProgress(rank: number, fraction: number, steps: number): number {
  if (!(steps > 0) || rank < 0) {
    return 0;
  }
  const bounded = Math.max(0, Math.min(1, fraction));
  return Math.max(0, Math.min(100, ((rank + bounded) / steps) * 100));
}

/**
 * A stable color derived from an identifier.
 *
 * Used to give a player a recognizable badge without calling an avatar
 * service: a UUID leaving would go to a third party, and this launcher only
 * lets out what it must. Two different accounts get two different hues, and
 * the same one on every launch.
 */
export function hue(identifier: string): number {
  let sum = 0;
  for (const character of identifier) {
    sum = (sum * 31 + character.charCodeAt(0)) % 360;
  }
  return sum;
}

/** The first two letters of a username, capitalized. */
export function initials(username: string): string {
  return username.trim().slice(0, 2).toUpperCase() || '?';
}
