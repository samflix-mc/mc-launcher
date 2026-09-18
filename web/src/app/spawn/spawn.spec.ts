import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Report, PackState } from '../core/contracts';
import { News } from '../core/news';
import { Pack } from '../core/pack';
import { Spawn } from './spawn';

function state(over: Partial<PackState> = {}): PackState {
  return {
    action: 'play',
    drift: 'up-to-date',
    offline: false,
    installed: true,
    name: 'samflix',
    version: '1.4.2',
    java: 21,
    mods: 128,
    generation: 1,
    ...over,
  };
}

function report(over: Partial<Report> = {}): Report {
  return {
    verdict: 'The session ended normally.',
    caughtUp: false,
    missing: [],
    drifts: [],
    offline: false,
    purge: [],
    ...over,
  };
}

/**
 * The Spawn page.
 *
 * What it must show fits in one sentence: the pack's state, legibly, and
 * what the last install left behind — without any of what the bottom bar
 * already carries.
 */
describe('Spawn', () => {
  let pack: Pack;

  function mount() {
    const fixture = TestBed.createComponent(Spawn);
    fixture.detectChanges();
    return fixture;
  }

  function read(fixture: ReturnType<typeof mount>, test: string): string | null {
    const element = fixture.nativeElement.querySelector(`[data-test="${test}"]`);
    return element ? element.textContent.replace(/\s+/g, ' ').trim() : null;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
    pack = TestBed.inject(Pack);
  });

  /**
   * **The badge says the state in three words, and never through color
   * alone.**
   *
   * The design system explicitly forbids it: `success` and `danger` are
   * only told apart by hue, and a label is what makes the difference for
   * someone who perceives it poorly.
   */
  it('up to date: the badge says so, with the version', () => {
    pack.state.set(state());
    const fixture = mount();

    expect(read(fixture, 'badge')).toContain('Up to date');
    expect(read(fixture, 'badge')).toContain('1.4.2');
  });

  it('not installed: the badge warns', () => {
    pack.state.set(state({ drift: 'absent', installed: false }));
    const fixture = mount();

    const badge = fixture.nativeElement.querySelector('[data-test="badge"]');
    expect(badge.textContent).toContain('Not installed');
    expect(badge.dataset.state).toBe('warning');
  });

  /**
   * Offline wins over drift: what's believed about the published pack
   * hasn't been checked, and calling it "up to date" would be a claim we
   * have no means to make.
   */
  it('offline wins over drift', () => {
    pack.state.set(state({ offline: true, drift: 'up-to-date' }));
    const fixture = mount();

    const badge = fixture.nativeElement.querySelector('[data-test="badge"]');
    expect(badge.textContent).toContain('Offline');
    expect(badge.dataset.state).toBe('offline');
  });

  it('before the first response, the badge says it’s looking', () => {
    const fixture = mount();

    const badge = fixture.nativeElement.querySelector('[data-test="badge"]');
    expect(badge.dataset.state).toBe('unknown');
    expect(badge.textContent).toContain('Checking');
  });

  it('the pack’s facts show up once known', () => {
    pack.state.set(state());
    const fixture = mount();

    expect(read(fixture, 'mods')).toContain('128 mods');
    expect(read(fixture, 'java')).toContain('Java 21');
    expect(read(fixture, 'installed')).toContain('Installed');
  });

  /**
   * **The cinematic isn't here anymore**, and this test keeps it that way.
   *
   * The button carries the progress, its percentage and its rate, and the
   * phrase above it names the step: a list of the eleven phases would say
   * it a third time, at the cost of a panel that appears and disappears
   * right where progress is being followed.
   */
  it('no step list, even while busy', () => {
    pack.state.set(state());
    pack.busy.set(true);
    pack.path.set([{ phase: 'mods', label: 'Mods', rank: 0 }]);
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="path"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="steps-panel"]')).toBeNull();
  });

  /**
   * The report only shows up IF it has something to say. A session without
   * incident leaves a verdict no one needs to read, and an empty panel
   * would take up the spot the image must keep.
   */
  it('a session without incident leaves no panel', () => {
    pack.state.set(state());
    pack.lastReport.set(report());
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="session-panel"]')).toBeNull();
  });

  it('missing mods are said, with their names', () => {
    pack.state.set(state());
    pack.lastReport.set(report({ missing: ['sodium', 'iris'] }));
    const fixture = mount();

    expect(read(fixture, 'missing')).toContain('2 mod(s) missing');
    expect(read(fixture, 'missing')).toContain('sodium');
  });

  it('a purge is said, and reassures about what was kept', () => {
    pack.state.set(state());
    pack.lastReport.set(report({ purge: ['mods', 'config'] }));
    const fixture = mount();

    expect(read(fixture, 'purge')).toContain('mods, config');
    expect(read(fixture, 'purge')).toContain('worlds');
  });

  /**
   * Without a news feed, the grid would keep a hole. A panel that says so
   * holds the spot and claims nothing.
   */
  it('without news, the spot is held by a panel that says so', () => {
    TestBed.inject(News);
    const fixture = mount();

    expect(read(fixture, 'no-news')).toContain('Nothing published');
  });
});
