import { Injectable, computed, inject, signal } from '@angular/core';

import { Notifications } from './notifications';
import { errorMessage } from './bridge';

/**
 * What went wrong, and that the player must see.
 *
 * ## One single place, and one at a time
 *
 * An error blurs everything else and blocks clicks: impossible to miss it,
 * where a banner at the bottom of the page went unnoticed as soon as you'd
 * scrolled.
 *
 * One at a time, and it's the last one that wins: stacking errors would ask
 * the player to close them one by one, when the first is almost always the
 * cause of the ones that follow.
 *
 * ## Why the overlay isn't in the menu
 *
 * The menu carries a `backdrop-filter`, which creates a stacking context AND
 * makes the `aside` a containing block for its `position: fixed` descendants.
 * An overlay living inside it would therefore be bounded by the menu,
 * whatever its `inset`. It's mounted at the `App` level, and that's the
 * constraint to keep.
 */
@Injectable({ providedIn: 'root' })
export class Incidents {
  /** The message shown, or `null`. */
  private readonly notifications = inject(Notifications);

  readonly current = signal<string | null>(null);

  readonly open = computed(() => this.current() !== null);

  /** Reports what just failed. Translates what Rust sent. */
  report(cause: unknown): void {
    const message = errorMessage(cause);
    // The browser console keeps the raw cause: the shown message is
    // formatted for a player, and we want both.
    console.error('[incident]', cause);
    this.current.set(message);
    // Centered, but WITHOUT a toast: the dialog already shows the incident
    // large, and a toast on top would say the same thing twice at the same
    // moment. What we gain is the trace: the dialog closes, the center
    // keeps it.
    this.notifications.archive('danger', 'Something went wrong', message);
  }

  close(): void {
    this.current.set(null);
  }

  /**
   * Wraps a call: reports the error, and renders `null` instead of
   * rejecting.
   *
   * Every call did the same sequence, each time with a chance to forget the
   * `catch` — and a promise rejected into the void, with an error no one
   * sees.
   */
  async guard<T>(action: () => Promise<T>): Promise<T | null> {
    this.close();
    try {
      return await action();
    } catch (cause) {
      this.report(cause);
      return null;
    }
  }
}
