import { TestBed } from '@angular/core/testing';
import { Router, provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { App } from './app';
import { ROUTES } from './routes';
import { Session } from './core/session';

/**
 * The shell.
 *
 * Three layouts, and it's the route that decides: the sign-in window, the
 * three-row page — navigation, content, bottom bar — and the two-row one,
 * which drops the bottom bar and gives its height to the body.
 */
describe('App', () => {
  let router: Router;
  let session: Session;

  /**
   * Mounts the shell AND clears the boot screen.
   *
   * The boot screen holds a FLOOR of four hundred milliseconds — without
   * it, a session already cached would flash it for a single frame. A test
   * that didn't wait for it would only ever see the splash screen, and
   * would conclude the shell never renders.
   */
  async function mount() {
    const fixture = TestBed.createComponent(App);
    fixture.detectChanges();
    await new Promise((resolve) => setTimeout(resolve, 600));
    await fixture.whenStable();
    fixture.detectChanges();
    return fixture;
  }

  /**
   * Goes to a route and lets the shell redraw.
   *
   * The account is set JUST BEFORE: outside any backend, `session.open()`
   * resets the account to null — that's its intended behavior, and
   * `start()` calls it on mount. Without this reset, the guards would find
   * an empty session and send everything back to `/signin`.
   */
  async function goTo(fixture: Awaited<ReturnType<typeof mount>>, url: string, signedIn = true) {
    session.account.set(
      signedIn ? { username: 'thesam1798', uuid: '0123', ownsTheGame: true } : null,
    );
    await router.navigateByUrl(url);
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter(ROUTES)] });
    router = TestBed.inject(Router);
    session = TestBed.inject(Session);
    session.account.set({ username: 'thesam1798', uuid: '0123', ownsTheGame: true });
  });

  /**
   * **Outside any backend, the screen says so** instead of failing on an
   * `invoke` that doesn't exist. This is the only case where this warning
   * appears.
   */
  it('with no backend reachable, the window says so', async () => {
    const fixture = await mount();

    expect(fixture.nativeElement.querySelector('[data-test="outside-tauri"]')).not.toBeNull();
  });

  it('the title bar is there from the first frame', async () => {
    const fixture = await mount();

    expect(fixture.nativeElement.querySelector('[data-test="title-bar"]')).not.toBeNull();
  });

  /**
   * Spawn carries the three rows: navigation, content, and a bottom bar
   * holding both the play button AND the player badge.
   */
  it('on Spawn, the page has three rows and the play button', async () => {
    const fixture = await mount();
    await goTo(fixture, '/spawn');

    const page = fixture.nativeElement.querySelector('[data-test="page"]');
    expect(page.classList).toContain('hm-page--three-rows');
    expect(fixture.nativeElement.querySelector('[data-test="nav"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="playbar"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="player-badge"]')).not.toBeNull();
  });

  /** News keeps the player badge, and nothing in the center. */
  it('on News, the badge stays but not the button', async () => {
    const fixture = await mount();
    await goTo(fixture, '/news');

    expect(fixture.nativeElement.querySelector('[data-test="bottom-bar"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="playbar"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="player-badge"]')).not.toBeNull();
  });

  /**
   * **Settings drops the bottom bar**, and the body takes the height: it's
   * an explicit design system rule, and it's what gives the settings list
   * room to scroll.
   */
  it('on Settings, the bottom bar disappears', async () => {
    const fixture = await mount();
    await goTo(fixture, '/settings');

    const page = fixture.nativeElement.querySelector('[data-test="page"]');
    expect(page.classList).toContain('hm-page--two-rows');
    expect(fixture.nativeElement.querySelector('[data-test="bottom-bar"]')).toBeNull();
  });

  /**
   * **Sign-in has no shell at all.**
   *
   * No navigation, no play button, no player badge: showing a menu and a
   * "sign out" button to someone who isn't signed in was the first
   * complaint the acceptance check raised.
   */
  it('on sign-in, there is neither navigation nor a bottom bar', async () => {
    session.account.set(null);
    const fixture = await mount();
    await goTo(fixture, '/signin', false);

    expect(fixture.nativeElement.querySelector('[data-test="page"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="nav"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="bottom-bar"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="sheet"]')).not.toBeNull();
  });

  /**
   * The sign-in window doesn't resize: letting the cursor promise a gesture
   * that nothing executes is worse than not promising anything.
   */
  it('on sign-in, the edges do not promise resizing', async () => {
    session.account.set(null);
    const fixture = await mount();
    await goTo(fixture, '/signin', false);

    expect(fixture.nativeElement.querySelector('[data-test="edges"]')).toBeNull();
  });

  /** Elsewhere, they're there — that's what the cursor was missing. */
  it('elsewhere, the eight edges carry the cursor', async () => {
    const fixture = await mount();
    await goTo(fixture, '/spawn');

    const edges = fixture.nativeElement.querySelector('[data-test="edges"]');
    expect(edges).not.toBeNull();
    expect(edges.children).toHaveLength(8);
  });

  /** The scene carries the image: it's what gives the glass something to blur. */
  it('the scene is always there, beneath everything else', async () => {
    const fixture = await mount();

    expect(fixture.nativeElement.querySelector('[data-test="scene"]')).not.toBeNull();
  });
});
