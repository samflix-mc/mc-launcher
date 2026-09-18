import { Injectable } from '@angular/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

import type {
  Progress,
  DeviceCode,
  Account,
  Folder,
  Screen,
  StepView,
  PackState,
  Feed,
  Brand,
  Report,
  ServerStatus,
  Settings,
} from './contracts';
import { IN_TAURI, call, listen, openOutsideApp } from './transport';

const EVENT_CODE = 'auth://code';
const EVENT_PROGRESS = 'cinematic://progress';

/**
 * What Rust emits toward the MAIN window once sign-in has succeeded.
 *
 * It loaded its front end before the session existed — while the player was
 * authenticating in another window — and its session service therefore
 * carries a null account. Without this signal, it would show a page it has
 * no reason to display anymore. Written on both sides: see `windows.rs`.
 */
const EVENT_SESSION = 'session-opened';

/**
 * The ONLY place that talks to Rust.
 *
 * Everything goes through here for a precise reason: outside the Tauri
 * window — `ng serve` alone, a test suite — `invoke` doesn't exist. No
 * component needs to know it; they read `available`, or go through a
 * `core/` service that handles it.
 *
 * This service holds NO rules. It calls and it returns. Everything that
 * decides lives either in a core service, or — preferably — in Rust, where
 * the mutation reaches.
 */
@Injectable({ providedIn: 'root' })
export class Bridge {
  /**
   * Is there anyone to talk to?
   *
   * True in the Tauri window, and ALSO true in a browser when the
   * development server is running — that's what lets you view the
   * interface anywhere other than the window. False in a test suite,
   * where there's neither.
   */
  readonly available = IN_TAURI || developmentServer();

  /**
   * Are we in the WINDOW, and not just in front of a backend?
   *
   * The distinction was born from a crash: `available` used to mean "in
   * Tauri" until the development server existed, and now means "there's
   * someone to talk to." But `getCurrentWindow()` doesn't address a
   * backend: it's a window API, and outside of one it throws.
   *
   * Everything that drives the WINDOW — minimize, maximize, close — reads
   * this, then; everything that requests DATA reads `available`.
   */
  readonly inWindow = IN_TAURI;

  // --- Windows ---------------------------------------------------------

  /**
   * Opens the sign-in window, and clears the main one.
   *
   * Called from the main window as soon as it knows the session isn't
   * playable. Outside of Tauri — in a browser, in front of the development
   * server — there's only one tab: the command does nothing, and the
   * router takes over to reach the page.
   */
  openSignIn(): Promise<void> {
    return call<void>('open_sign_in');
  }

  /**
   * The session is open: the main window takes back control, the sign-in
   * one leaves.
   */
  signInSucceeded(): Promise<void> {
    return call<void>('sign_in_succeeded');
  }

  /**
   * The main window announces that it has something to display.
   *
   * Rust WAITS for it before swapping windows: showing it earlier would
   * expose a screen filling itself in over two seconds, which the sign-in
   * window covers by staying legible in its place.
   */
  mainReady(): Promise<void> {
    return call<void>('main_ready');
  }

  /** The main window learns that a session just opened elsewhere. */
  onSessionOpened(receive: () => void): Promise<UnlistenFn> {
    return listen<unknown>(EVENT_SESSION, () => receive());
  }

  // --- The log -----------------------------------------------------------

  /**
   * Writes a line to Rust's log, under this window's label.
   *
   * The label isn't sent: Rust reads it off the calling window. It's the
   * only point in the chain where you can't get the window wrong — and
   * getting the window wrong is exactly the defect this log exists to
   * track.
   */
  log(level: string, message: string): Promise<void> {
    return call<void>('log', { level, message });
  }

  // --- Startup -------------------------------------------------------------

  /**
   * Tells Rust the front end has finished rendering.
   *
   * That's what closes the boot screen — a second window, without a
   * script, open while Angular loads — and shows the main window, hidden
   * until then.
   *
   * Rust can't guess it: it knows when a window EXISTS, not when its
   * content is painted. The front end is the only one that knows.
   */
  frontReady(): Promise<void> {
    return call<void>('front_ready');
  }

  // --- Launcher identity -----------------------------------------------

  brand(): Promise<Brand> {
    return call<Brand>('brand');
  }

  path(): Promise<StepView[]> {
    return call<StepView[]>('path');
  }

  // --- Session -------------------------------------------------------------

  status(): Promise<Account | null> {
    return call<Account | null>('status');
  }

  /** Opens a Microsoft session. Returns only once the code has been validated. */
  signIn(): Promise<Account> {
    return call<Account>('sign_in');
  }

  signOut(): Promise<void> {
    return call<void>('sign_out');
  }

  /**
   * Subscribes to the device code.
   *
   * It arrives as an event and not as a return value: `signIn()` waits for
   * the player to have authorized, and the code has to be displayed during
   * that wait.
   */
  onDeviceCode(receive: (code: DeviceCode) => void): Promise<UnlistenFn> {
    return listen<DeviceCode>(EVENT_CODE, receive);
  }

  // --- The pack ------------------------------------------------------------

  /** What the disk and the published pack say. A few dozen KiB. */
  packState(): Promise<PackState> {
    return call<PackState>('pack_state');
  }

  /**
   * Places the pack, and stops there.
   *
   * **Doesn't launch the game**, and that comes back from acceptance
   * testing: placing eight hundred megabytes and playing are two separate
   * intents, and the second doesn't follow from the first. Progress
   * arrives by event throughout.
   */
  install(): Promise<Report> {
    return call<Report>('install');
  }

  /**
   * Verifies, catches up if needed, then launches the session.
   *
   * Only returns at the end of the session. Verification stays up front:
   * entering with NeoForge registries that no longer match shows up as a
   * kick at connection time, with no useful message.
   */
  play(): Promise<Report> {
    return call<Report>('play');
  }

  /**
   * Stops the running session, without ceremony.
   *
   * For a game that no longer responds: it's the only case where this
   * gesture is useful, and also the one where asking nicely doesn't work.
   * `play()` will return shortly after, with the report of an interrupted
   * session.
   */
  stopGame(): Promise<void> {
    return call<void>('stop_game');
  }

  verifyFiles(deep: boolean): Promise<string[]> {
    return call<string[]>('verify_files', { deep });
  }

  onProgress(receive: (progress: Progress) => void): Promise<UnlistenFn> {
    return listen<Progress>(EVENT_PROGRESS, receive);
  }

  // --- News ------------------------------------------------------------

  news(): Promise<Feed> {
    return call<Feed>('news');
  }

  // --- The server ------------------------------------------------------

  /**
   * Probes the server this pack declares for this binary's environment.
   *
   * Never rejects on an unreachable server or a missing declaration — both
   * are ordinary states the panel draws, not failures. A few seconds at
   * most: the command carries its own short deadline.
   */
  serverStatus(): Promise<ServerStatus> {
    return call<ServerStatus>('server_status');
  }

  // --- Settings --------------------------------------------------------

  settings(): Promise<Settings> {
    return call<Settings>('settings');
  }

  /** Returns what was actually WRITTEN, not what was sent. See `mc-settings`. */
  saveSettings(settings: Settings): Promise<Settings> {
    return call<Settings>('save_settings', { settings });
  }

  screen(): Promise<Screen | null> {
    return call<Screen | null>('screen');
  }

  openFolder(what: Folder): Promise<void> {
    return call<void>('open_folder', { what });
  }

  // --- The system ----------------------------------------------------------

  /**
   * Opens a URL in the SYSTEM's browser.
   *
   * Never in the window: it's a privileged origin where `invoke` is
   * reachable, and Rust's navigation plugin would refuse it anyway. It's
   * the exit path for every link in a post.
   */
  openPage(url: string): Promise<void> {
    return openOutsideApp(url);
  }
}

/**
 * The player's head, rendered in three dimensions by mc-heads.net.
 *
 * A third-party service, and it's a choice: the UUID leaves. It's public —
 * any server the player connects to knows it — and that's the price of a
 * real skin render rather than a color badge. `tauri.conf.json`'s CSP
 * allows this domain, and only this one.
 */
export function playerHead(uuid: string, size = 96): string {
  return `https://mc-heads.net/head/${encodeURIComponent(uuid)}/${size}`;
}

/**
 * The message of an error coming from Rust.
 *
 * `Error` serializes to a string; whatever comes back otherwise comes from
 * the bridge itself, and we don't want to show a player "[object Object]".
 */
export function errorMessage(cause: unknown): string {
  if (typeof cause === 'string') {
    return cause;
  }
  if (cause instanceof Error) {
    return cause.message;
  }
  try {
    return JSON.stringify(cause) ?? String(cause);
  } catch {
    return String(cause);
  }
}

/**
 * Is the development server supposed to be responding?
 *
 * There's no way to know without asking it, and `available` is read
 * synchronously by the whole interface. So it relies on context: a browser
 * serving the front end from the loopback address is a development
 * machine, and that's the only case where the server exists.
 *
 * In a test suite — jsdom — `location.hostname` is `localhost` but there's
 * no server: calls will fail, and that's intended. The tests that rely on
 * the bridge's absence go through `core/` services, which read `available`
 * and stay quiet. That's why `jsdom` is excluded explicitly.
 */
function developmentServer(): boolean {
  if (typeof window === 'undefined' || navigator.userAgent.includes('jsdom')) {
    return false;
  }
  return ['localhost', '127.0.0.1'].includes(window.location.hostname);
}
