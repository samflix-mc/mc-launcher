import { inject } from '@angular/core';
import { Router, type CanActivateFn, type Routes } from '@angular/router';

import { Session } from './core/session';

/**
 * Can the player access the launcher's pages?
 *
 * ## The predicate is UNIQUE, and that's the whole point
 *
 * Both guards read `session.playable()`, and nothing else. Two different
 * predicates — "signed in" on one side, "playable" on the other — would
 * bounce a signed-in account WITHOUT A LICENSE back and forth forever: the
 * pages guard would send it to `/signin`, the sign-in guard would find it
 * signed in and send it back to the pages, and so on until the router gives
 * up.
 *
 * This case isn't theoretical: it's a valid Microsoft account that has
 * never bought Minecraft. It's handled as a STATE of the Sign-in page, with
 * its own message, and not as a redirect.
 *
 * ## Why the guard waits
 *
 * `open()` queries Rust, which takes the time of two network round-trips.
 * The router WAITS for a guard that returns a promise and validates no URL
 * while it's pending: there's therefore no jump to prevent, which is why no
 * `withDisabledInitialNavigation()` is set.
 */
const playable: CanActivateFn = async () => {
  const session = inject(Session);
  const router = inject(Router);

  if (!session.known()) {
    await session.open();
  }
  return session.playable() ? true : router.createUrlTree(['/signin']);
};

/** The reverse, on EXACTLY the same predicate. */
const notYetPlayable: CanActivateFn = async () => {
  const session = inject(Session);
  const router = inject(Router);

  if (!session.known()) {
    await session.open();
  }
  return session.playable() ? router.createUrlTree(['/spawn']) : true;
};

/**
 * The routes.
 *
 * ## The `bottom` data
 *
 * It says what the window's bottom bar carries: the play button and the
 * player badge, the badge alone, or nothing — in which case the page
 * occupies two rows instead of three. It's route DATA and not a test on the
 * URL: a string compared to `'/spawn'` breaks the day a route gains a
 * parameter, and the symptom is an empty bottom bar that nothing explains.
 *
 * ## Everything is lazy, without exception
 *
 * `loadComponent` on each one: a page's chunk is only downloaded once you
 * navigate to it. On a launcher, this shows — the settings page and the
 * news page are only opened one time in ten, and loading them at startup
 * would delay the screen everyone is looking at.
 *
 * ## `pathMatch: 'full'` on the empty redirect
 *
 * Without it, Angular refuses the route with NG04014: a redirect from an
 * empty path without `pathMatch` is ambiguous, since the empty path is a
 * prefix of everything.
 */
export const ROUTES: Routes = [
  {
    path: 'signin',
    canActivate: [notYetPlayable],
    // Sign-in is a modal over the scene: no shell at all, so no bottom
    // bar. See `app.html`.
    data: { bottom: 'none' },
    loadComponent: () => import('./signin/signin').then((m) => m.SignIn),
  },
  {
    path: 'spawn',
    canActivate: [playable],
    data: { bottom: 'play' },
    loadComponent: () => import('./spawn/spawn').then((m) => m.Spawn),
  },
  {
    path: 'news',
    canActivate: [playable],
    // The player badge stays, the play button doesn't: the design system
    // keeps the bottom bar on News, with nothing in the center.
    data: { bottom: 'player' },
    loadComponent: () => import('./news/news').then((m) => m.NewsPage),
  },
  {
    path: 'settings',
    canActivate: [playable],
    // Settings DROPS the bottom bar, and the body takes the height: it's
    // an explicit design system rule, and it's what gives the settings
    // list room to scroll.
    data: { bottom: 'none' },
    loadComponent: () => import('./settings/settings').then((m) => m.Settings),
  },
  { path: '', pathMatch: 'full', redirectTo: 'spawn' },
  // An unknown path must not leave an empty window: in a desktop
  // application, there's no address bar to escape from.
  { path: '**', redirectTo: 'spawn' },
];
