/**
 * What Rust serializes, written once and read everywhere.
 *
 * These types have NO code: they describe the bridge contract. Keeping them
 * apart from the services is what lets you read them at a glance when
 * looking for what a command returns — and what keeps a component from
 * importing an entire service for a single type.
 *
 * Each name is written on both sides. Renaming it on only one leaves a
 * window that no longer displays anything, without any compilation
 * complaining: it's JSON, and JSON never complains.
 */

/** Under what name the launcher presents itself, fixed at compile time. */
export interface Brand {
  readonly name: string;
  readonly seal: string;
}

/** The signed-in account. */
export interface Account {
  readonly username: string;
  readonly uuid: string;
  /**
   * False when the account has no Minecraft Java Edition license. Sign-in
   * still succeeds — it's a valid Microsoft account — but no online server
   * will accept it.
   */
  readonly ownsTheGame: boolean;
}

/** What the player has to enter on Microsoft's side. */
export interface DeviceCode {
  readonly code: string;
  readonly url: string;
  /** The same page, with the code prefilled. This is the one that gets opened. */
  readonly directUrl: string;
}

/** The phase identifiers, as `phase.rs` serializes them. */
export type Phase =
  | 'signin'
  | 'license'
  | 'pack'
  | 'loader'
  | 'minecraft'
  | 'java'
  | 'neo-forge'
  | 'mods'
  | 'lock'
  | 'ready'
  | 'launch';

/** A phase of the path, with what's needed to render it. */
export interface StepView {
  readonly phase: Phase;
  readonly label: string;
  readonly rank: number;
}

/** The installation's state, five times a second. */
export interface Progress {
  readonly phase: Phase;
  /** Is the phase a reached state rather than work in progress? */
  readonly done: boolean;
  readonly note: string | null;
  readonly file: string | null;
  readonly bytes: number;
  readonly total: number;
  readonly files: number;
  readonly filesTotal: number;
  /** Is a download actually in progress? */
  readonly active: boolean;
  readonly rate: number;
  readonly remaining: number | null;
}

/** What the button should say. */
export type Action = 'install' | 'play';

/** What separates what's placed from what's published. */
export type Drift = 'absent' | 'up-to-date' | 'update' | 'reinstall' | 'unknown';

/** What the disk and the published pack say, without installing anything. */
export interface PackState {
  readonly action: Action;
  readonly drift: Drift;
  readonly offline: boolean;
  readonly installed: boolean;
  readonly name: string | null;
  readonly version: string | null;
  readonly java: number | null;
  readonly mods: number;
  readonly generation: number;
}

/**
 * What a gesture left behind.
 *
 * **The same type for installing and for playing**: both leave the same
 * traces — missing mods, drifts from the lock, a purge — and only the
 * `verdict` differs. Two twin types would force the screen to carry two
 * display paths to say the same thing.
 */
export interface Report {
  readonly verdict: string;
  /** Was anything actually placed? */
  readonly caughtUp: boolean;
  readonly missing: readonly string[];
  readonly drifts: readonly string[];
  readonly offline: boolean;
  /** What the purge erased, if there was a purge. */
  readonly purge: readonly string[];
}

// --- News --------------------------------------------------------------

/**
 * A fragment of text within a post.
 *
 * A union DISCRIMINATED on `type`: that's what `#[serde(tag = "type")]`
 * produces on the Rust side, and it's what lets the template write an
 * ordinary `@switch`. Without the tag, serde would render `{"Bold": {...}}`
 * and you'd have to inspect an object's first key.
 */
export type Inline =
  | { readonly type: 'text'; readonly text: string }
  | { readonly type: 'bold'; readonly text: string }
  | { readonly type: 'italic'; readonly text: string }
  | { readonly type: 'code'; readonly text: string }
  | { readonly type: 'link'; readonly text: string; readonly href: string };

/** A top-level block of a post. */
export type Block =
  | { readonly type: 'paragraph'; readonly content: readonly Inline[] }
  | { readonly type: 'heading'; readonly level: number; readonly content: readonly Inline[] }
  | { readonly type: 'list'; readonly bullets: readonly (readonly Inline[])[] }
  | { readonly type: 'separator' };

export interface Post {
  readonly id: string;
  readonly title: string;
  /** RFC 3339 in UTC. Already validated by Rust. */
  readonly date: string;
  readonly pinned: boolean;
  /** A `data:` URI built by Rust, or `null`. Never a remote URL. */
  readonly image: string | null;
  /**
   * The body, as a typed tree. **Never HTML**: that's the condition under
   * which the CSP was loosened, and the reason no `innerHTML` exists
   * anywhere in this repo.
   */
  readonly body: readonly Block[];
}

export interface Feed {
  readonly posts: readonly Post[];
  /** The feed comes from the last known copy, not from the network. */
  readonly offline: boolean;
  /** The posts that were discarded, and why. */
  readonly discarded: readonly string[];
}

// --- Settings ------------------------------------------------------------

export type WindowMode = 'windowed' | 'maximized' | 'fullscreen';
export type Backdrop = 'spawn' | 'nether' | 'end' | 'plain';

export interface GameSettings {
  renderDistance: number;
  simulationDistance: number;
  maxFps: number;
  guiScale: number;
  vsync: boolean;
}

export interface WindowSettings {
  mode: WindowMode;
  width: number;
  height: number;
}

export interface LauncherSettings {
  memoryMb: number | null;
  minimizeOnLaunch: boolean;
}

export interface AppearanceSettings {
  backdrop: Backdrop;
  scrim: number;
}

export interface Settings {
  schema: number;
  game: GameSettings;
  window: WindowSettings;
  launcher: LauncherSettings;
  appearance: AppearanceSettings;
}

/** What the player's screen allows — work AREA, panels deduced from it. */
export interface Screen {
  readonly width: number;
  readonly height: number;
  readonly scale: number;
}

/** The folders offered for opening. A closed list, on the Rust side too. */
export type Folder = 'data' | 'config' | 'logs' | 'instance';
