import {
  ChangeDetectionStrategy,
  Component,
  type OnInit,
  afterNextRender,
  computed,
  inject,
  signal,
} from '@angular/core';
import { NavigationEnd, Router, RouterOutlet, type ActivatedRouteSnapshot } from '@angular/router';

import { Boot } from './boot/boot';
import { TitleBar } from './shell/title-bar/title-bar';
import { Incident } from './shell/incident/incident';
import { Player } from './shell/player/player';
import { Nav } from './shell/nav/nav';
import { Notifications as NotificationsPanel } from './shell/notifications/notifications';
import { Playbar } from './shell/playbar/playbar';
import { WindowService } from './core/window';
import { Incidents } from './core/incidents';
import { Log } from './core/log';
import { Brand } from './core/brand';
import { Pack } from './core/pack';
import { Bridge } from './core/bridge';
import { SettingsService } from './core/settings';
import { Session } from './core/session';

/**
 * The floor of the Angular boot screen, in milliseconds.
 *
 * Now that a real splash window covers loading — see
 * `crates/mc-app/src/startup.rs` — this boot screen only covers the NETWORK
 * wait, after the window has shown. Four hundred stays a floor, because it
 * needs one: without it, a session already cached would flash the boot
 * screen for a single frame.
 */
const BOOT_FLOOR_MS = 400;

/**
 * The delay between the first render and the signal sent to Rust.
 *
 * `afterNextRender` fires when Angular has written to the DOM — not when the
 * browser has PAINTED. Showing the window at that exact instant would make
 * it appear on a still-empty frame, which would replace a clean splash
 * screen with a flash.
 */
const BEFORE_SHOWING_MS = 250;

/** What the bottom bar carries, depending on the page. See `routes.ts`. */
export type Bottom = 'play' | 'player' | 'none';

/** The route that lives in its own window. */
const SIGNIN_ROUTE = '/signin';

/**
 * The shell: the window, the scene, the title bar, the page.
 *
 * ## The frame comes from the design system, as-is
 *
 * `.hm-window` contains `.hm-stage` — the image and its readability
 * gradient — then `.hm-titlebar`, which floats above it, then `.hm-page`,
 * whose three rows are the centered navigation pill, the content, and the
 * bottom bar. The MIDDLE of the page is left empty on purpose: that's where
 * the image shows through, and it's the only spot on screen where it's
 * really seen.
 *
 * ## The boot screen doesn't cover the title bar
 *
 * It occupies the content area, below the bar. `status()` chains two
 * network round-trips: on a slow network or behind a captive portal, a
 * full-screen boot screen would leave a window with no system button — we
 * removed those — and no app button — they'd be underneath. Acceptance
 * check: open the packaged build WITH NO NETWORK, and close it during boot.
 *
 * ## As long as the session isn't playable, there's no shell
 *
 * No navigation pill, no bottom bar, no player badge: the sign-in page is a
 * modal over the scene, and nothing behind it is reachable. Showing a menu
 * and a "sign out" button to someone who isn't signed in was the first
 * complaint the acceptance check raised.
 */
@Component({
  selector: 'app-root',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterOutlet, Boot, TitleBar, Nav, Player, Playbar, Incident, NotificationsPanel],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App implements OnInit {
  // All injected as FIELDS and not inside `start()`: `inject()` is only
  // usable in an injection context, and an async method leaves it as soon
  // as the first `await` runs. The error only shows up at runtime, as an
  // NG0203 that doesn't name the offending line.
  private readonly bridge = inject(Bridge);
  private readonly session = inject(Session);
  private readonly incidents = inject(Incidents);
  private readonly brandService = inject(Brand);
  private readonly settings = inject(SettingsService);
  private readonly windowService = inject(WindowService);
  private readonly pack = inject(Pack);
  private readonly router = inject(Router);
  private readonly trace = inject(Log);

  /** False outside the Tauri window AND outside the dev server. */
  protected readonly available = this.bridge.available;

  /** Is the boot screen still showing? */
  protected readonly boot = signal(true);

  protected readonly brand = this.brandService.view;
  protected readonly maximized = this.windowService.maximized;
  protected readonly playable = this.session.playable;

  /**
   * Are we on the sign-in page?
   *
   * It isn't drawn like the others: it's its own WINDOW, four hundred and
   * forty pixels wide, with a full-surface frosted sheet and a title bar
   * with no maximize button. The shell — navigation, play button, player
   * badge — doesn't exist there.
   */
  protected readonly onSignIn = signal(false);

  /**
   * What the bottom bar carries, read from the current route.
   *
   * From the route's DATA, and not from a test on the URL: a string
   * compared to `'/spawn'` breaks the day a route gains a parameter, and the
   * symptom is an empty bottom bar that nothing explains.
   */
  protected readonly bottom = signal<Bottom>('none');

  /** True when the page occupies all three rows, false when it occupies two. */
  protected readonly threeRows = computed(() => this.bottom() !== 'none');

  constructor() {
    this.trace.step(
      `shell mounted — label "${this.windowService.label ?? 'outside-tauri'}", ` +
        `main=${this.windowService.isMain}, dedicated=${this.windowService.inADedicatedWindow}`,
    );

    this.router.events.subscribe((event) => {
      if (!(event instanceof NavigationEnd)) {
        return;
      }
      this.bottom.set(this.bottomOf(this.router.routerState.snapshot.root));

      const onSignIn = event.urlAfterRedirects.startsWith(SIGNIN_ROUTE);
      this.onSignIn.set(onSignIn);

      // THE line to read when a window shows a page that isn't its own: it
      // says who navigated, to what, and from which URL.
      this.trace.step(
        `navigation done — "${event.url}" → "${event.urlAfterRedirects}", ` +
          `onSignIn=${onSignIn}, bottom=${this.bottom()}`,
      );

      // The MAIN window never shows sign-in: it delegates it to a dedicated
      // window, and hides itself behind it. It stays on this route —
      // nobody sees it, since it's hidden — and will take back control once
      // the session signal fires.
      //
      // The sign-in window, on the other hand, IS already on this route: it
      // would only open itself.
      if (onSignIn && this.windowService.isMain) {
        void this.bridge.openSignIn().catch(() => {});
      }
    });

    // The signal that closes the splash screen and shows the window.
    //
    // It does NOT depend on `start()`, and that's deliberate: that one
    // queries the network, which can take a while behind a captive portal.
    // Waiting on its data to show the window would leave the player facing
    // a splash screen with no button at all.
    afterNextRender(() => {
      setTimeout(() => {
        this.trace.step('first render: "front_ready" sent');
        void this.bridge.frontReady().catch(() => {});
      }, BEFORE_SHOWING_MS);
    });
  }

  ngOnInit(): void {
    // When the session opens in the other window, this one has to learn
    // about it: its session service carries a null account since it
    // loaded, and nothing would tell it otherwise.
    //
    // **Only a window that isn't dedicated subscribes to this**, and it's a
    // belt on top of `transport.ts`'s suspenders. Rust emits to "main" by
    // name; the JS listener registered on `Any` regardless, which Tauri
    // serves with NO filter — so the sign-in window would receive the
    // signal, navigate to Spawn, and draw it in four hundred and forty
    // pixels right before disappearing.
    //
    // The targeting is fixed at the source; this guard also states what the
    // rule WANTS, right where you read it.
    if (this.windowService.inADedicatedWindow) {
      this.trace.detail('dedicated window: not listening for "session-opened"');
    } else {
      void this.bridge
        .onSessionOpened(() => {
          this.trace.step('"session-opened" received');
          void this.resumeControl();
        })
        .catch(() => {});
    }

    void this.start();
  }

  /**
   * What happens during the boot screen.
   *
   * All in parallel: the brand, the settings, the window state and the
   * session start together. Chaining them would turn the boot screen into
   * the sum of four latencies instead of the largest one.
   *
   * `finally` and not just the success branch: if `status()` fails —
   * network down, unreadable token — the boot screen must still clear,
   * otherwise the launcher stays stuck on its splash screen with no visible
   * error.
   */
  private async start(): Promise<void> {
    const floor = new Promise((resolve) => setTimeout(resolve, BOOT_FLOOR_MS));

    try {
      await Promise.all([
        this.brandService.load(),
        this.settings.load(),
        this.windowService.watch(),
        this.session.open(),
        // The pack state belongs to the SHELL and not to Spawn: the title
        // bar draws the modpack name from it, and the play button draws
        // everything else. Opening it from Spawn made the name disappear
        // as soon as you changed page.
        this.pack.open(),
      ]);
    } catch (cause) {
      this.trace.concern(`startup failed: ${String(cause)}`);
      this.incidents.report(cause);
    } finally {
      await floor;
      this.boot.set(false);
      this.trace.step(`boot screen cleared — playable=${this.session.playable()}`);
    }
  }

  /**
   * The session just opened in the sign-in window.
   *
   * We re-read the account, then go to Spawn: the main window had stayed on
   * `/signin` while it was hidden, and leaving it there would show a
   * sign-in page to someone who just signed in.
   */
  private async resumeControl(): Promise<void> {
    this.trace.step('resuming control: re-reading the session');
    await this.incidents.guard(async () => {
      await this.session.open();
      this.trace.detail(`session re-read — playable=${this.session.playable()}`);
      await this.pack.refresh();
      this.trace.detail('pack state refreshed; navigating to Spawn');
      const navigated = await this.router.navigate(['/spawn']);
      this.trace.step(`navigation to Spawn: ${navigated ? 'accepted' : 'REFUSED'}`);
    });

    // We do NOT say we're ready here: the home page says so, from its own
    // `afterNextRender`. `navigate` returns control once the route is
    // activated, which precedes the first pixel — and Rust would then show
    // a still-empty window.
    //
    // If the navigation failed, nobody will say so: the guard delay of
    // `sign_in_succeeded` switches after eight seconds rather than leaving
    // a "Signed in" that leads nowhere.
  }

  /** The `bottom` data of the deepest route, or "none" if there isn't one. */
  private bottomOf(root: ActivatedRouteSnapshot): Bottom {
    let node: ActivatedRouteSnapshot | undefined = root;
    let found: Bottom = 'none';
    while (node) {
      const value = node.data['bottom'] as Bottom | undefined;
      if (value) {
        found = value;
      }
      node = node.firstChild ?? undefined;
    }
    return found;
  }
}
