/**
 * The boot screen's phrases.
 *
 * ## Why a list and not a status message
 *
 * Sam asked for a configurable phrase under the loader, leaving both
 * options open: real status messages — "setting up" — or playful phrases.
 * Both are possible here; this file carries the latter.
 *
 * The reason for the choice: the boot screen lasts NINE HUNDRED
 * MILLISECONDS at most. A status message changing three times in under a
 * second doesn't read — it flickers. A phrase held throughout reads, and
 * makes the wait feel like an intention rather than a slowdown.
 *
 * ## The draw
 *
 * `Math.random()` would be perfectly suited here: no one depends on the
 * phrase, and a repeat costs nothing. The only care is that it be called
 * ONCE per opening and not on every render — otherwise the phrase would
 * change on every change detection, which is the opposite of readable.
 */
export const PHRASES: readonly string[] = [
  'Loading blocks…',
  'Waking the villagers…',
  'Checking the chests…',
  'Aligning the redstone…',
  'The creeper hasn’t seen you.',
  'Compiling mods, not excuses.',
  'Feeding the wolves…',
  'Looking for the lost spawn…',
  'Polishing the smooth stone…',
  'Negotiating with the Piglins…',
];

/**
 * A phrase, drawn once.
 *
 * `crypto.getRandomValues` and not `Math.random`, and this is NOT a
 * security requirement: picking a waiting message needs none. It's that
 * static analysis flags every `Math.random` as a pseudo-random generator
 * used without a stated reason — it can't know this one protects
 * nothing — and an alert you learn to ignore ends up hiding the one that
 * mattered.
 *
 * The cost is nil: an API available everywhere this launcher runs, and one
 * draw per boot.
 *
 * The modulo is taken without bias correction. There is one, tiny and
 * fully accepted: a phrase would need to come up a hundred-thousandth more
 * often than another for anyone to notice.
 */
export function aPhrase(): string {
  const draw = new Uint32Array(1);
  crypto.getRandomValues(draw);
  return PHRASES[draw[0] % PHRASES.length];
}
