import { ApplicationConfig, provideBrowserGlobalErrorListeners } from '@angular/core';
import { provideRouter, withComponentInputBinding } from '@angular/router';

import { ROUTES } from './routes';

/**
 * The strict minimum, and the options that aren't obvious.
 *
 * ## No `withHashLocation()` — real paths
 *
 * The routes are `/spawn`, `/news`, `/settings`. There used to be a
 * fragment — `#/spawn` — whose motive was caution: owe nothing to Tauri's
 * asset protocol's SPA fallback, which was treated as an implementation
 * detail rather than a contract.
 *
 * It cost more than it protected. The hash belongs to the router the
 * moment it's in play, so no anchor can serve any other purpose on the
 * page anymore: the Settings rail paid for it — clicking
 * `#video-settings` wrote a URL the router tried to resolve as a route,
 * and navigation went back to `/spawn`.
 *
 * The fallback, though, is indeed there, and verified in the sources of
 * the pinned version: `tauri-2.11.5/src/manager/mod.rs` chains four
 * attempts for a missing asset — `<path>.html`, `<path>/index.html`, then
 * `index.html`. And `ng serve` has always done the same. Both environments
 * this launcher runs in therefore serve `index.html` for an unknown route.
 *
 * What to check if the window ever opened blank: it's this fourth attempt
 * that would need checking, and `<base href="/">` in `index.html`, without
 * which assets would resolve against `/spawn/`.
 *
 * The navigation plugin changes nothing here: its predicate is on the
 * ORIGIN, never on the path.
 *
 * ## No `withDisabledInitialNavigation()`
 *
 * It would serve to prevent a screen jump while the guards query Rust. But
 * there is no jump: a guard can return a promise, the router waits for it,
 * and no URL is validated while it's pending.
 *
 * Worse, the flag alone does NOTHING useful — it sets a token, and
 * `router.initialNavigation()` has to be called by hand; forgetting that
 * gives a window stuck on its splash screen, with no error in the console.
 * And it removes the only overlap available: lazy chunks are only resolved
 * after the guards, so strictly after the handshake.
 *
 * ## `withComponentInputBinding()`
 *
 * URL parameters arrive as a component `input()`, with no page having to
 * subscribe to `ActivatedRoute`. Nothing uses it yet; it's one line now
 * rather than a migration the day a page takes a post identifier.
 */
export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    provideRouter(ROUTES, withComponentInputBinding()),
  ],
};
