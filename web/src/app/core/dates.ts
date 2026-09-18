/**
 * Say the date of a post.
 *
 * ## Why not `Intl.RelativeTimeFormat` alone
 *
 * "3 days ago" reads well for what's recent and badly for the rest:
 * "47 days ago" asks for mental arithmetic that "August 2" doesn't. So we
 * switch over at a week, which is roughly the horizon where people stop
 * counting.
 *
 * ## A pure function, and a tested one
 *
 * It takes the reference instant as a parameter. Without that, we'd have to
 * fake the clock to exercise it, and the tests would depend on the day
 * they're run.
 */
export function sayTheDate(iso: string, now: Date = new Date()): string {
  const then = new Date(iso);
  if (Number.isNaN(then.getTime())) {
    // Rust already validates the format, but a feed served by a modified
    // host could pass something else. Showing "Invalid Date" would be worse
    // than saying nothing.
    return '';
  }

  const seconds = Math.round((now.getTime() - then.getTime()) / 1000);
  const relative = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

  if (seconds < 60) {
    return 'just now';
  }
  if (seconds < 3600) {
    return relative.format(-Math.round(seconds / 60), 'minute');
  }
  if (seconds < 86_400) {
    return relative.format(-Math.round(seconds / 3600), 'hour');
  }
  if (seconds < 7 * 86_400) {
    return relative.format(-Math.round(seconds / 86_400), 'day');
  }

  // Beyond a week: a date, with the year only when it isn't the current
  // one — "August 2, 2024" when it's useful, "August 2" otherwise.
  const sameYear = then.getUTCFullYear() === now.getUTCFullYear();
  return new Intl.DateTimeFormat('en', {
    day: 'numeric',
    month: 'long',
    year: sameYear ? undefined : 'numeric',
    timeZone: 'UTC',
  }).format(then);
}
