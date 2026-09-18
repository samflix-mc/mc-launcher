import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { WindowService } from '../core/window';
import { Bridge } from '../core/bridge';
import { Spawn } from './spawn';

/**
 * **Who has the right to say the main window is ready.**
 *
 * Rust waits for this signal to swap windows at the end of a sign-in. It
 * used to be sent for days by the SIGN-IN window: it received an event that
 * wasn't meant for it — see `transport.target.spec.ts` — mounted Spawn in
 * its four-hundred-and-forty pixels, and thereby announced that a window was
 * ready. The wrong one.
 *
 * The targeting is fixed upstream. This guard is the second line: even if a
 * dedicated window mounted Spawn for a reason we didn't foresee, it
 * wouldn't speak in the main one's name.
 */
describe('Spawn, the window-ready signal', () => {
  let mainReady: ReturnType<typeof vi.fn>;

  function mount(inADedicatedWindow: boolean) {
    mainReady = vi.fn(async () => {});
    TestBed.configureTestingModule({
      providers: [
        provideRouter([]),
        {
          provide: Bridge,
          useValue: {
            available: false,
            inWindow: false,
            mainReady,
            log: vi.fn(async () => {}),
          },
        },
        {
          provide: WindowService,
          useValue: { inADedicatedWindow, isMain: !inADedicatedWindow },
        },
      ],
    });
    const fixture = TestBed.createComponent(Spawn);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
  });

  it('in the main window, it departs', async () => {
    const fixture = mount(false);
    await fixture.whenStable();

    expect(mainReady).toHaveBeenCalled();
  });

  it('in a dedicated window, it does NOT depart', async () => {
    const fixture = mount(true);
    await fixture.whenStable();

    expect(mainReady).not.toHaveBeenCalled();
  });
});
