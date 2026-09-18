import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Bridge } from './bridge';
import { WindowService } from './window';

describe('WindowService', () => {
  let windowService: WindowService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    windowService = TestBed.inject(WindowService);
  });

  it('believes itself windowed until told otherwise', () => {
    expect(windowService.maximized()).toBe(false);
  });

  /**
   * **Outside Tauri, the four gestures do nothing — and above all don't
   * throw.**
   *
   * This isn't a comfort detail: the title bar is mounted by the shell, so
   * it also exists under `ng serve`, where `getCurrentWindow()` has no one
   * to talk to. Without the `available` guard, clicking the close button
   * there would produce a rejected promise that would surface in the
   * incident overlay — an error message for a button that has nothing to
   * do.
   */
  it('outside Tauri, window gestures do nothing', async () => {
    await expect(windowService.minimize()).resolves.toBeUndefined();
    await expect(windowService.toggleMaximized()).resolves.toBeUndefined();
    await expect(windowService.close()).resolves.toBeUndefined();
    await expect(windowService.watch()).resolves.toBeUndefined();

    // And the state hasn't moved: nothing got maximized.
    expect(windowService.maximized()).toBe(false);
  });

  /**
   * **The regression that motivated this test.**
   *
   * `available` used to mean "inside Tauri" until the dev server existed;
   * it now means "there's a backend". In the browser, it's therefore TRUE —
   * and the guard that protected `getCurrentWindow()` stopped protecting.
   * The symptom: a `TypeError: Cannot read properties of undefined (reading
   * 'metadata')` at startup, surfaced in the incident overlay before even
   * the first screen.
   *
   * This service drives the WINDOW: it must therefore read `inWindow`,
   * not `available`. The test checks this by setting up exactly the
   * browser's situation — a reachable backend, no window.
   */
  it('does not touch the window when there is a backend but no window', async () => {
    const bridge = TestBed.inject(Bridge);
    // The case of a browser pointed at the dev server.
    Object.defineProperty(bridge, 'available', { value: true, configurable: true });
    Object.defineProperty(bridge, 'inWindow', { value: false, configurable: true });

    await expect(windowService.watch()).resolves.toBeUndefined();
    await expect(windowService.minimize()).resolves.toBeUndefined();
    await expect(windowService.toggleMaximized()).resolves.toBeUndefined();
    await expect(windowService.close()).resolves.toBeUndefined();
  });
});
