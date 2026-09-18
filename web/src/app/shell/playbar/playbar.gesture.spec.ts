import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { PackState } from '../../core/contracts';
import { Pack } from '../../core/pack';
import { Bridge } from '../../core/bridge';
import { Playbar } from './playbar';

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

/**
 * **What the click actually triggers.**
 *
 * The label and the gesture must say the same thing, and that's the whole
 * point of the acceptance feedback: the button used to announce "Update and
 * play" and launched Minecraft right after. A test on the label alone
 * wouldn't have caught anything — it's the call that matters.
 */
describe('Playbar, the gesture', () => {
  let install: ReturnType<typeof vi.fn>;
  let play: ReturnType<typeof vi.fn>;
  let stopGame: ReturnType<typeof vi.fn>;
  let pack: Pack;

  const outcome = {
    verdict: 'The pack is installed.',
    caughtUp: true,
    missing: [],
    drifts: [],
    offline: false,
    purge: [],
  };

  beforeEach(() => {
    TestBed.resetTestingModule();
    install = vi.fn(async () => outcome);
    play = vi.fn(async () => ({ ...outcome, verdict: 'Session ended.' }));
    stopGame = vi.fn(async () => {});
    TestBed.configureTestingModule({
      providers: [
        {
          provide: Bridge,
          // **`packState` is not optional here, even though this test never
          // reads it.** Every gesture ends with `Pack.duringGesture`
          // rereading the disk, so a fake without it throws
          // `packState is not a function` inside the promise the click
          // returned — which never settles, and the test dies on the 5s
          // timeout instead of on a readable assertion.
          //
          // `available` stays `true`: dropping it to `false` would silence
          // the refresh, but it also guards `stopGame`, and one of these
          // four tests is precisely about `stopGame` being called.
          useValue: {
            available: true,
            inWindow: false,
            install,
            play,
            stopGame,
            packState: vi.fn(async () => state()),
          },
        },
      ],
    });
    pack = TestBed.inject(Pack);
  });

  async function click(laid: Partial<PackState>) {
    pack.state.set(state(laid));
    const fixture = TestBed.createComponent(Playbar);
    fixture.detectChanges();
    fixture.nativeElement.querySelector('[data-test="button"]').click();
    // One loop turn is enough: the gesture is an already-resolved promise.
    // `whenStable()` would instead wait for the whole app to settle — which
    // a shell component listening to a stream never gives it.
    await new Promise((resume) => setTimeout(resume, 0));
  }

  it("when there's something to lay down, it LAYS DOWN — and doesn't play", async () => {
    await click({ action: 'install', drift: 'update' });

    expect(install).toHaveBeenCalledOnce();
    expect(play).not.toHaveBeenCalled();
  });

  it('when everything is up to date, it plays', async () => {
    await click({ action: 'play', drift: 'up-to-date' });

    expect(play).toHaveBeenCalledOnce();
    expect(install).not.toHaveBeenCalled();
  });

  /** Nothing laid down: it installs, obviously — and doesn't play either. */
  it('on an empty disk, it installs', async () => {
    await click({ action: 'install', drift: 'absent', installed: false });

    expect(install).toHaveBeenCalledOnce();
    expect(play).not.toHaveBeenCalled();
  });

  /**
   * **Only the second click stops the game.**
   *
   * The first asks the question. Killing Minecraft loses whatever wasn't
   * saved, and this button sits at the center of the bottom bar: without
   * this pause, a stray click would cost a whole game session.
   */
  it('stopping the game asks for two clicks', async () => {
    pack.state.set(state());
    pack.progress.set({
      phase: 'launch',
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
    });
    const fixture = TestBed.createComponent(Playbar);
    fixture.detectChanges();
    const button = fixture.nativeElement.querySelector('[data-test="button"]');

    button.click();
    fixture.detectChanges();
    expect(stopGame).not.toHaveBeenCalled();

    button.click();
    await new Promise((resume) => setTimeout(resume, 0));
    expect(stopGame).toHaveBeenCalledOnce();
  });
});
