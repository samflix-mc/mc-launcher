import { Injectable, computed, inject, signal } from '@angular/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

import type { Progress, Report, StepView, PackState, Phase } from './contracts';
import { Bridge } from './bridge';
import * as format from './format';

/**
 * The phases that are a STATE and not work.
 *
 * They close the path and don't count toward progress: "ready to play"
 * isn't a step you execute, it's the result of the others.
 */
const TERMINAL_PHASES: ReadonlySet<Phase> = new Set(['ready', 'launch']);

/** Where a phase of the path stands, from the display's point of view. */
export type State = 'done' | 'in-progress' | 'upcoming';

/** What the button displays, activity included. */
export type Button =
  | 'unknown' // still looking
  | 'install'
  | 'play'
  | 'busy' // catching up
  | 'in-game';

/**
 * The pack's state, and what the button says about it.
 *
 * ## Why the button's state has five values and not three
 *
 * The `Action` Rust returns says what THE DISK requires: install or play.
 * It says nothing about ACTIVITY — an installation in progress, a running
 * session — because activity isn't read off a disk, it's observed.
 *
 * Conflating the two would answer "an installation is already in progress"
 * to someone clicking during their session, which is the longest state of
 * the session.
 *
 * And "unknown" is a necessary fifth value: between Spawn's first render
 * and `packState()`'s response, an "Install" default would produce an
 * INSTALL → PLAY flicker on every startup, on the one screen element that
 * matters.
 */
@Injectable({ providedIn: 'root' })
export class Pack {
  private readonly bridge = inject(Bridge);

  readonly state = signal<PackState | null>(null);
  readonly path = signal<StepView[]>([]);
  readonly progress = signal<Progress | null>(null);
  /** What the last gesture — install or play — left behind. */
  readonly lastReport = signal<Report | null>(null);

  /** True while catching up or the session runs. */
  readonly busy = signal(false);

  /**
   * True while the game runs, which is longer than everything else.
   *
   * DERIVED from progress and not set by hand: a signal you have to
   * remember to reset ends up stuck at `true` one day, and the button
   * stays "In game" for the rest of the session. Here, the cinematic
   * leaves "launch" for "ready" at the end of the session, and the
   * computation follows.
   */
  readonly inGame = computed(() => this.progress()?.phase === 'launch');

  readonly button = computed<Button>(() => {
    if (this.inGame()) {
      return 'in-game';
    }
    if (this.busy()) {
      return 'busy';
    }
    const state = this.state();
    if (!state) {
      return 'unknown';
    }
    return state.action === 'install' ? 'install' : 'play';
  });

  /** How many steps actually count toward progress. */
  private readonly usefulSteps = computed(
    () => this.path().filter((step) => !TERMINAL_PHASES.has(step.phase)).length,
  );

  /**
   * The path annotated with each phase's state.
   *
   * Computed from the RANK rather than from a list kept up to date: the
   * received phase is enough to know what's behind and what's left, and
   * nothing can drift out of sync.
   */
  readonly steps = computed(() => {
    const seen = this.progress();
    const rank = this.rankOf(seen?.phase);

    // A "done" phase is behind us, not in progress: "ready to play"
    // shouldn't flicker as if it were still being waited on.
    const inProgress = seen && !seen.done ? rank : -1;
    let lastDone = -1;
    if (seen) {
      lastDone = seen.done ? rank : rank - 1;
    }

    return this.path().map((step) => {
      let state: State = 'upcoming';
      if (step.rank <= lastDone) {
        state = 'done';
      } else if (step.rank === inProgress) {
        state = 'in-progress';
      }
      return { ...step, state };
    });
  });

  /** The progress of the WHOLE installation, not just the current batch. */
  readonly overallProgress = computed(() => {
    const seen = this.progress();
    if (!seen) {
      return 0;
    }
    const fraction = format.batchFraction(seen.bytes, seen.total, seen.files, seen.filesTotal);
    return format.overallProgress(this.rankOf(seen.phase), fraction, this.usefulSteps());
  });

  /** The current step, as it's written: "4 / 9 · Mods". */
  readonly currentStep = computed(() => {
    const seen = this.progress();
    if (!seen || seen.done) {
      return null;
    }
    const step = this.path().find((candidate) => candidate.phase === seen.phase);
    return step ? { number: step.rank + 1, total: this.usefulSteps(), label: step.label } : null;
  });

  private unsubscribe: UnlistenFn | null = null;

  /**
   * On open: the path to draw, the subscription, then the pack's state.
   *
   * The progress subscription is set up ONCE and for good, not for every
   * session: an event emitted between two subscriptions would be lost, and
   * the bar would stay frozen until the next one.
   */
  async open(): Promise<void> {
    if (!this.bridge.available) {
      return;
    }
    if (!this.unsubscribe) {
      this.unsubscribe = await this.bridge.onProgress((p) => this.progress.set(p));
      this.path.set(await this.bridge.path());
    }
    await this.refresh();
  }

  /** Asks again for the pack's state. A few dozen KiB, no disk touched. */
  async refresh(): Promise<void> {
    if (!this.bridge.available) {
      return;
    }
    this.state.set(await this.bridge.packState());
  }

  /**
   * Places whatever there is to place, and stops there.
   *
   * The button's gesture when something is missing or has changed. It
   * doesn't launch the game: that's a second click, on a button that will
   * then say "Play".
   */
  async install(): Promise<Report> {
    return this.duringGesture(() => this.bridge.install());
  }

  /**
   * Verifies, catches up if needed, then launches the session.
   *
   * `busy` covers the whole call; `inGame` only lights up once catch-up is
   * done — that is, once the cinematic reaches "launch". Distinguishing
   * the two is what lets the button say "Installing…" and then "In game"
   * rather than an indistinct "Busy" for twenty minutes.
   */
  async play(): Promise<Report> {
    return this.duringGesture(() => this.bridge.play());
  }

  /**
   * Stops the running session.
   *
   * Does NOT touch `busy`: the `play()` call is still in flight and will
   * return on its own, with the report of an interrupted session. Lowering
   * it here would make the button clickable during the second the game
   * dies, and a second click would target a session that no longer exists.
   */
  async stopGame(): Promise<void> {
    if (!this.bridge.available) {
      return;
    }
    await this.bridge.stopGame();
  }

  /**
   * What the two gestures have in common.
   *
   * Extracted because the three steps — raising `busy`, keeping the
   * report, rereading the disk — have to be the same: copying them would
   * one day leave one of the two forgetting to refresh, and the button
   * would keep the label from before the installation it just did.
   */
  private async duringGesture(gesture: () => Promise<Report>): Promise<Report> {
    this.busy.set(true);
    try {
      const report = await gesture();
      this.lastReport.set(report);
      // The disk has changed: the previous state no longer holds.
      await this.refresh();
      return report;
    } finally {
      this.busy.set(false);
    }
  }

  /** The rank of a phase in the path, or -1 if it isn't there. */
  private rankOf(phase: Phase | undefined): number {
    return this.path().find((step) => step.phase === phase)?.rank ?? -1;
  }
}
