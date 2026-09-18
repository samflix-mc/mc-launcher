import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Progress, PackState } from './contracts';
import { Pack } from './pack';

function state(partial: Partial<PackState>): PackState {
  return {
    action: 'play',
    drift: 'up-to-date',
    offline: false,
    installed: true,
    name: 'samflix',
    version: '3.2',
    java: 21,
    mods: 120,
    generation: 0,
    ...partial,
  };
}

function progress(partial: Partial<Progress>): Progress {
  return {
    phase: 'mods',
    done: false,
    note: null,
    file: null,
    bytes: 0,
    total: 0,
    files: 0,
    filesTotal: 0,
    active: false,
    rate: 0,
    remaining: null,
    ...partial,
  };
}

describe('Pack — the button', () => {
  let pack: Pack;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    pack = TestBed.inject(Pack);
  });

  /**
   * THE button test, and it's about the case that gets forgotten.
   *
   * Between Spawn's render and `packState()`'s response, the button has
   * to say it's LOOKING. An "Install" default would produce an
   * INSTALL → PLAY flicker on every startup, on the one screen element
   * that matters.
   */
  it('says "unknown" until the state is known', () => {
    expect(pack.button()).toBe('unknown');
  });

  it('follows what the disk requires', () => {
    pack.state.set(state({ action: 'install' }));
    expect(pack.button()).toBe('install');

    pack.state.set(state({ action: 'play' }));
    expect(pack.button()).toBe('play');
  });

  /**
   * ACTIVITY wins over action.
   *
   * The action says what the disk requires; it says nothing about an
   * installation in progress. Conflating them would offer PLAY while
   * downloading.
   */
  it('busy wins over action', () => {
    pack.state.set(state({ action: 'play' }));
    pack.busy.set(true);
    expect(pack.button()).toBe('busy');
  });

  /**
   * And the SESSION wins over everything — it's the longest state of the
   * session, and the one no field of the model carried before.
   */
  it('the session wins over busy', () => {
    pack.state.set(state({ action: 'play' }));
    pack.busy.set(true);
    pack.progress.set(progress({ phase: 'launch' }));
    expect(pack.button()).toBe('in-game');
  });

  /**
   * `inGame` is DERIVED and not set: a signal you have to remember to
   * reset ends up stuck at `true` one day, and the button stays "In game"
   * for the rest of the session.
   */
  it('leaves "in game" once the cinematic returns to "ready"', () => {
    pack.progress.set(progress({ phase: 'launch' }));
    expect(pack.inGame()).toBe(true);

    pack.progress.set(progress({ phase: 'ready', done: true }));
    expect(pack.inGame()).toBe(false);
  });
});

describe('Pack — the cinematic', () => {
  let pack: Pack;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    pack = TestBed.inject(Pack);
    pack.path.set([
      { phase: 'pack', label: 'Pack', rank: 0 },
      { phase: 'mods', label: 'Mods', rank: 1 },
      { phase: 'ready', label: 'Ready', rank: 2 },
    ]);
  });

  it('marks what is behind, in progress, and upcoming', () => {
    pack.progress.set(progress({ phase: 'mods', done: false }));

    const states = pack.steps().map((s) => s.state);
    expect(states).toEqual(['done', 'in-progress', 'upcoming']);
  });

  /**
   * A DONE phase is behind us, not in progress: "ready to play" shouldn't
   * flicker as if it were still being waited on.
   */
  it('a done phase is behind, not in progress', () => {
    pack.progress.set(progress({ phase: 'mods', done: true }));

    const states = pack.steps().map((s) => s.state);
    expect(states).toEqual(['done', 'done', 'upcoming']);
  });

  /**
   * "Ready" is a TERMINAL phase: it doesn't count toward progress, because
   * it isn't a step you execute but the result of the others.
   */
  it('terminal phases do not count toward progress', () => {
    pack.progress.set(progress({ phase: 'pack', total: 100, bytes: 100, active: true }));
    // Two useful steps out of three: finishing the first makes 50%.
    expect(Math.round(pack.overallProgress())).toBe(50);
  });

  /**
   * **The bar moves during resolution, which weighs nothing.**
   *
   * Thirty-six seconds of API polling on Sam's pack, without a single byte
   * announced: progress stayed stuck at the phase's rank, and the screen
   * looked no different from a crashed one.
   */
  it('without announced bytes, progress follows the request count', () => {
    const pack = TestBed.inject(Pack);
    pack.path.set([
      { phase: 'mods', label: 'Mods', rank: 0 },
      { phase: 'lock', label: 'Lock', rank: 1 },
    ]);

    pack.progress.set(progress({ phase: 'mods', bytes: 0, total: 0, files: 0, filesTotal: 51 }));
    const start = pack.overallProgress();

    pack.progress.set(progress({ phase: 'mods', bytes: 0, total: 0, files: 25, filesTotal: 51 }));

    expect(pack.overallProgress()).toBeGreaterThan(start);
  });
});
