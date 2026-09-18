import { Injectable, computed, signal } from '@angular/core';

/** The tone of a notification, as the design system names them. */
export type Tone = 'success' | 'danger' | 'info' | 'warning' | 'progress';

/** What just happened. */
export interface Notice {
  readonly id: number;
  readonly tone: Tone;
  /** What happened, in five words. */
  readonly title: string;
  /** The number, or the steps to follow. One line. */
  readonly text: string | null;
  /** The instant, in RFC 3339 — the same format as posts. */
  readonly date: string;
  readonly read: boolean;
}

/**
 * How many notices the center keeps.
 *
 * Fifty, as the design system sets it. Beyond that, a launcher left open for
 * a week would accumulate a log no one scrolls through, and whose only
 * effect would be to lengthen the panel's render.
 */
const MEMORY = 50;

/** How many toasts are shown at once. The others wait their turn. */
const VISIBLE_TOASTS = 3;

/**
 * How long a toast stays, by tone, in milliseconds.
 *
 * The durations come from the design system: six seconds for what informs,
 * twelve for what asks for a decision. `progress` isn't in it — a progress
 * toast stays until the work finishes, and it's the caller who removes it by
 * replacing it.
 */
const DURATIONS: Record<Tone, number | null> = {
  success: 6000,
  info: 6000,
  danger: 12_000,
  warning: 12_000,
  progress: null,
};

/**
 * The launcher's notifications, on the two levels the window carries.
 *
 * ## The design system's three levels, and the one that's missing
 *
 * The design system describes three: the toast, transient, bottom left; the
 * center, persistent, under the bell; and the SYSTEM notification, for the
 * same events when the launcher is hidden behind the game.
 *
 * The first two are here. The third can't be from the front: it needs
 * `@tauri-apps/plugin-notification`, so a Rust dependency, a permission in
 * `capabilities/default.json`, and a settings key "Notifications: launcher
 * and system / launcher only / none". That's backend work, and stating it
 * here rather than staying silent about it avoids anyone thinking the level
 * was forgotten.
 *
 * ## Every toast is ALSO written to the center
 *
 * That's the design system's rule, and it has a reason: a toast lasts six
 * seconds, and six seconds is enough to look away. Nothing that was said
 * should disappear without leaving a trace.
 *
 * The reverse isn't true: you can write to the center WITHOUT a toast —
 * that's what incidents do, which the window already shows in its dialog,
 * and which a toast would repeat on top of.
 */
@Injectable({ providedIn: 'root' })
export class Notifications {
  /** The log, most recent first. */
  readonly log = signal<readonly Notice[]>([]);

  /** The ids of the notices currently shown as a toast. */
  private readonly visible = signal<readonly number[]>([]);

  /** Is the panel under the bell open? */
  readonly panelOpen = signal(false);

  readonly unread = computed(() => this.log().filter((notice) => !notice.read).length);

  /**
   * The toasts, oldest to most recent.
   *
   * The design system wants the most recent AT THE BOTTOM: the stack grows
   * upward, and the eye always comes back to the same spot. `.hm-toasts`
   * aligns its content at the bottom, so the array order is enough.
   */
  readonly toasts = computed(() => {
    const seen = this.visible();
    return this.log()
      .filter((notice) => seen.includes(notice.id))
      .slice(0, VISIBLE_TOASTS)
      .reverse();
  });

  private next = 1;

  /**
   * An event that deserves a toast.
   *
   * Renders the id, which lets the caller remove it itself — a progress
   * toast, which is replaced when the work finishes.
   */
  notify(tone: Tone, title: string, text: string | null = null): number {
    const id = this.record(tone, title, text);
    this.visible.update((seen) => [id, ...seen]);

    const duration = DURATIONS[tone];
    if (duration !== null) {
      setTimeout(() => this.closeToast(id), duration);
    }
    return id;
  }

  /**
   * An event the window ALREADY shows some other way.
   *
   * It enters the center and opens no toast. That's the case for incidents:
   * the dialog shows them large, and a toast on top would say the same
   * thing twice at the same moment.
   */
  archive(tone: Tone, title: string, text: string | null = null): number {
    return this.record(tone, title, text);
  }

  /** Removes a toast from the screen. The notice stays in the center. */
  closeToast(id: number): void {
    this.visible.update((seen) => seen.filter((one) => one !== id));
  }

  togglePanel(): void {
    const open = !this.panelOpen();
    this.panelOpen.set(open);
    // Opening the panel means having seen what it carries. Marking on close
    // would leave the badge lit while it's being read underneath.
    if (open) {
      this.markAllRead();
    }
  }

  closePanel(): void {
    this.panelOpen.set(false);
  }

  markAllRead(): void {
    this.log.update((notices) => notices.map((one) => (one.read ? one : { ...one, read: true })));
  }

  empty(): void {
    this.log.set([]);
    this.visible.set([]);
  }

  private record(tone: Tone, title: string, text: string | null): number {
    const id = this.next++;
    const notice: Notice = {
      id,
      tone,
      title,
      text,
      date: new Date().toISOString(),
      read: false,
    };
    this.log.update((log) => [notice, ...log].slice(0, MEMORY));
    return id;
  }
}
