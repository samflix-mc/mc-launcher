import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Progress, PackState } from '../../core/contracts';
import { Pack } from '../../core/pack';
import { Playbar, fileName } from './playbar';

/** A plausible pack state, each test only changing what it cares about. */
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

/** A plausible progress, same principle. */
function progress(over: Partial<Progress> = {}): Progress {
  return {
    phase: 'mods',
    done: false,
    note: null,
    file: 'sodium.jar',
    bytes: 420_000_000,
    total: 840_000_000,
    files: 64,
    filesTotal: 128,
    active: true,
    rate: 8_400_000,
    remaining: 30,
    ...over,
  };
}

/**
 * The button, and the line above it.
 *
 * The tests go through the DOM and `data-test` attributes: that's the
 * repo's convention, and it's also what makes these tests describe what the
 * player READS rather than the component's internal shape.
 */
describe('Playbar', () => {
  let pack: Pack;

  function mount() {
    const fixture = TestBed.createComponent(Playbar);
    fixture.detectChanges();
    return fixture;
  }

  function read(fixture: ReturnType<typeof mount>, test: string): string | null {
    const element = fixture.nativeElement.querySelector(`[data-test="${test}"]`);
    return element ? element.textContent.replace(/\s+/g, ' ').trim() : null;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
    pack = TestBed.inject(Pack);
  });

  /**
   * **Between opening and `pack_state()`'s response, the button says it's
   * looking.**
   *
   * Defaulting to "Install" would produce an INSTALL → PLAY flicker on
   * every startup, on the one element of the screen that matters.
   */
  it('before it knows, it doesn\'t say "Install"', () => {
    const fixture = mount();

    expect(read(fixture, 'button')).toContain('Checking…');
    expect(read(fixture, 'hint')).toContain('Reading what');
  });

  it('nothing installed: it offers to install, and says why', () => {
    pack.state.set(state({ action: 'install', drift: 'absent', installed: false }));
    const fixture = mount();

    expect(read(fixture, 'button')).toContain('Install');
    expect(read(fixture, 'hint')).toContain('Not installed yet');
    expect(read(fixture, 'hint')).toContain('1.4.2');
  });

  it('pack up to date: it offers to play', () => {
    pack.state.set(state());
    const fixture = mount();

    expect(read(fixture, 'button')).toContain('Play');
    expect(read(fixture, 'hint')).toContain('Up to date');
  });

  /**
   * **The label SAYS what's about to happen, and nothing more.**
   *
   * It used to say "Update and play", and it did both: Sam clicked to lay
   * down a modpack and Minecraft started. The button lays down, stops, and
   * becomes "Play" — that's a second click, when the player decides.
   *
   * The `action` comes from Rust and is "install" as soon as there's
   * something to lay down; this test sets it the way `mc_pack::comparison`
   * would return it.
   */
  it("update pending: the button LAYS DOWN, it doesn't play", () => {
    pack.state.set(state({ action: 'install', drift: 'update' }));
    const fixture = mount();

    expect(read(fixture, 'button')).toContain('Update');
    expect(read(fixture, 'button')).not.toContain('play');
    expect(read(fixture, 'hint')).toContain('Update available');
  });

  it('generation changed: it announces a reinstall, and reassures', () => {
    pack.state.set(state({ action: 'install', drift: 'reinstall' }));
    const fixture = mount();

    expect(read(fixture, 'button')).toContain('Reinstall');
    expect(read(fixture, 'button')).not.toContain('play');
    expect(read(fixture, 'hint')).toContain('your worlds');
  });

  /**
   * **Offline AND nothing installed: there's nothing to launch and nothing
   * to download.** Offline with a pack laid down, on the other hand, plays
   * just fine — the launcher just couldn't check.
   */
  it('offline with nothing installed, the button is inert and says why', () => {
    pack.state.set(state({ offline: true, installed: false, action: 'install' }));
    const fixture = mount();

    const button = fixture.nativeElement.querySelector('[data-test="button"]');
    expect(button.disabled).toBe(true);
    expect(button.classList.contains('hm-play--disabled')).toBe(true);
    expect(read(fixture, 'hint')).toContain('Offline');
  });

  it('offline with a pack laid down, we play anyway', () => {
    pack.state.set(state({ offline: true, installed: true }));
    const fixture = mount();

    const button = fixture.nativeElement.querySelector('[data-test="button"]');
    expect(button.disabled).toBe(false);
    expect(read(fixture, 'hint')).toContain('updates aren');
  });

  /**
   * During work, the button carries the fill, the percentage and the rate —
   * and that's precisely what made it possible to remove the step list from
   * the Spawn page.
   */
  it('during the install, the button carries the progress', () => {
    pack.state.set(state());
    pack.busy.set(true);
    pack.path.set([
      { phase: 'mods', label: 'Mods', rank: 0 },
      { phase: 'lock', label: 'Finalizing', rank: 1 },
    ]);
    pack.progress.set(progress());
    const fixture = mount();

    expect(read(fixture, 'button')).toContain('Installing…');
    expect(read(fixture, 'meta')).toContain('MB/s');
    expect(read(fixture, 'hint')).toContain('Mods');

    const fill = fixture.nativeElement.querySelector('[data-test="fill"]');
    expect(fill.style.getPropertyValue('--p')).not.toBe('');
  });

  /**
   * **A progress we don't know reads as such.**
   *
   * The indeterminate variant slides; a bar at zero percent would claim to
   * know we've done nothing, which is false — we simply don't know.
   */
  it('when progress is unknown, the bar is indeterminate', () => {
    const fixture = mount();

    const fill = fixture.nativeElement.querySelector('[data-test="fill"]');
    expect(fill.classList.contains('hm-play__fill--indeterminate')).toBe(true);
  });

  /**
   * The session is the longest-running state. Conflating "busy" and
   * "in-game" would make it answer "an operation is already running" to
   * someone clicking while they're playing.
   *
   * **And the button is clickable now.** `play` only returns once the
   * session ends: a Minecraft stuck on a loading screen used to leave the
   * launcher stuck there, with no way out but the task manager.
   */
  it('during the session, the button says so — and stays clickable', () => {
    pack.state.set(state());
    pack.progress.set(progress({ phase: 'launch', active: false }));
    const fixture = mount();

    expect(read(fixture, 'button')).toContain('Playing');
    expect(fixture.nativeElement.querySelector('[data-test="button"]').disabled).toBe(false);
    expect(read(fixture, 'hint')).toContain('The game is running');
  });

  /**
   * **Stopping is asked for twice.**
   *
   * Killing the game loses whatever wasn't saved, and this button sits at
   * the center of the bottom bar: one click too many happens fast there.
   * The first click asks, only the second acts.
   */
  it('the first click on "Playing" asks for confirmation', () => {
    pack.state.set(state());
    pack.progress.set(progress({ phase: 'launch', active: false }));
    const fixture = mount();

    fixture.nativeElement.querySelector('[data-test="button"]').click();
    fixture.detectChanges();

    expect(read(fixture, 'button')).toContain('Stop the game?');
    expect(read(fixture, 'hint')).toContain("won't be saved");
  });

  /**
   * With no published version, the line must not carry an orphaned middle
   * dot — "Not installed yet ·" reads like a missing value.
   */
  it("with no known version, the line doesn't trail a separator", () => {
    pack.state.set(state({ drift: 'absent', version: null, installed: false }));
    const fixture = mount();

    expect(read(fixture, 'hint')).toBe('Not installed yet');
  });

  /**
   * **What's being downloaded, and at what speed.**
   *
   * The acceptance feedback was: "we roughly know what step we're on, but
   * we don't know what's being downloaded, at what speed, nor the percent
   * progress". All these values were emitted five times a second by
   * `Tracker` — none of them was shown.
   */
  it('during the install, the second line says everything we know', () => {
    pack.state.set(state({ action: 'install', drift: 'update' }));
    pack.busy.set(true);
    pack.progress.set(progress({ file: '/long/path/to/sodium.jar' }));
    const fixture = mount();

    const line = read(fixture, 'detail') ?? '';
    expect(line).toContain('sodium.jar');
    expect(line).not.toContain('/long/path');
    expect(line).toContain('64 of 128 files');
    expect(line).toContain('of 840');
    expect(line).toContain('/s');
    expect(line).toContain('remaining');
  });

  /**
   * **The percentage survives the gaps between batches.**
   *
   * `active` falls back to false during mod resolution, jar inspection and
   * the NeoForge installer — that is, a good chunk of the time. The button
   * used to show nothing at all then, which reads as a hang.
   *
   * The rate, though, disappears: it means nothing when nothing is coming
   * down, and a frozen number would look like a real measurement.
   */
  it('between batches, the percentage stays and the rate leaves', () => {
    pack.state.set(state({ action: 'install', drift: 'update' }));
    pack.busy.set(true);
    pack.path.set([{ phase: 'mods', label: 'Mods', rank: 0 }]);
    pack.progress.set(progress({ active: false, rate: 0 }));
    const fixture = mount();

    expect(read(fixture, 'meta')).toMatch(/\d+ %/);
    expect(read(fixture, 'meta')).not.toContain('/s');
  });

  /**
   * During VERIFICATION, there's neither a percentage nor a detail to show:
   * nothing has been measured yet. The line on top says so, and that's all.
   */
  it('during verification, no figure is made up', () => {
    const fixture = mount();

    expect(read(fixture, 'hint')).toContain('Reading what');
    expect(read(fixture, 'meta')).toBeNull();
    expect(read(fixture, 'detail')).toBeNull();
  });
});

/**
 * The bare name of a path.
 *
 * Tested separately because it's a free function: it needs neither a
 * component nor a DOM.
 */
describe('fileName', () => {
  it('keeps only the last portion', () => {
    expect(fileName('/a/b/c/sodium.jar')).toBe('sodium.jar');
    expect(fileName('sodium.jar')).toBe('sodium.jar');
    expect(fileName('C:\\games\\mods\\iris.jar')).toBe('iris.jar');
  });

  /** A path ending in a separator must not return an empty string. */
  it('a path ending in a separator returns the path', () => {
    expect(fileName('/a/b/')).toBe('/a/b/');
  });
});
