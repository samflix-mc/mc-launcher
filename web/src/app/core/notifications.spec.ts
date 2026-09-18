import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { Notifications } from './notifications';

describe('Notifications', () => {
  let service: Notifications;

  beforeEach(() => {
    vi.useFakeTimers();
    TestBed.configureTestingModule({});
    service = TestBed.inject(Notifications);
  });

  /**
   * **The design system's rule: every toast is ALSO written to the
   * center.**
   *
   * A toast lasts six seconds, and six seconds is enough to look away. If
   * `notify` only wrote to the screen, a download that failed while reading
   * the news would disappear without leaving a trace.
   */
  it('a toast also enters the center', () => {
    service.notify('success', 'Pack updated', '128 mods');

    expect(service.toasts()).toHaveLength(1);
    expect(service.log()).toHaveLength(1);
    expect(service.log()[0].title).toBe('Pack updated');
  });

  /**
   * The reverse isn't true, and that's what lets incidents avoid saying
   * things twice: the dialog already shows them large.
   */
  it('archive writes to the center without opening a toast', () => {
    service.archive('danger', 'Something went wrong', 'detail');

    expect(service.toasts()).toHaveLength(0);
    expect(service.log()).toHaveLength(1);
  });

  /**
   * The durations come from the design system: six seconds for what
   * informs, twelve for what asks for a decision. An error toast that
   * cleared in six seconds would take the only thing worth reading with it.
   */
  it('a success toast leaves after six seconds, a danger toast holds for twelve', () => {
    service.notify('success', 'Done');
    service.notify('danger', 'Failed');

    vi.advanceTimersByTime(6000);
    expect(service.toasts().map((notice) => notice.title)).toEqual(['Failed']);

    vi.advanceTimersByTime(6000);
    expect(service.toasts()).toEqual([]);
    // Gone from the screen, still in the center.
    expect(service.log()).toHaveLength(2);
  });

  /**
   * A progress toast stays until the work finishes: it's the caller who
   * removes it. Seeing it disappear after six seconds during an
   * eight-hundred-megabyte download would be the opposite of what it
   * announces.
   */
  it('a progress toast does not leave on its own', () => {
    const id = service.notify('progress', 'Downloading');

    vi.advanceTimersByTime(60_000);
    expect(service.toasts()).toHaveLength(1);

    service.closeToast(id);
    expect(service.toasts()).toEqual([]);
  });

  /**
   * **The most recent AT THE BOTTOM.**
   *
   * `.hm-toasts` aligns its content at the bottom and the stack grows
   * upward: the eye always comes back to the same spot. The array order
   * carries this rule, and reversing it would make new toasts appear at the
   * top of the stack — right where no one is looking.
   */
  it('toasts are rendered oldest to most recent', () => {
    service.notify('info', 'First');
    service.notify('info', 'Second');
    service.notify('info', 'Third');

    expect(service.toasts().map((notice) => notice.title)).toEqual(['First', 'Second', 'Third']);
  });

  /** Three on screen at most; the rest wait in the center. */
  it('at most three toasts are shown at once', () => {
    for (let rank = 1; rank <= 5; rank += 1) {
      service.notify('info', `Notice ${rank}`);
    }

    expect(service.toasts()).toHaveLength(3);
    expect(service.log()).toHaveLength(5);
  });

  /**
   * Opening the panel means having seen what it carries. Marking on CLOSE
   * would leave the badge lit while it's being read underneath.
   */
  it('opening the panel marks everything as read', () => {
    service.notify('info', 'One');
    service.notify('info', 'Two');
    expect(service.unread()).toBe(2);

    service.togglePanel();

    expect(service.panelOpen()).toBe(true);
    expect(service.unread()).toBe(0);
  });

  /**
   * Fifty, as the design system sets it. Without a bound, a launcher left
   * open for a week would accumulate a log no one scrolls through.
   */
  it('the center keeps only the last fifty', () => {
    for (let rank = 1; rank <= 60; rank += 1) {
      service.archive('info', `Notice ${rank}`);
    }

    expect(service.log()).toHaveLength(50);
    // Most recent first: it's the sixtieth that remains, not the first.
    expect(service.log()[0].title).toBe('Notice 60');
  });

  it('empty clears the center and the toasts', () => {
    service.notify('info', 'One');
    service.empty();

    expect(service.log()).toEqual([]);
    expect(service.toasts()).toEqual([]);
  });
});
