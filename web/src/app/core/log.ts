import { Injectable, inject } from '@angular/core';

import { Bridge } from './bridge';

/** The levels Rust knows how to read back. See `crates/mc-app/src/log.rs`. */
export type Level = 'error' | 'warn' | 'info' | 'debug' | 'trace';

/**
 * What the front tells Rust's log.
 *
 * ## Why not `console.log`
 *
 * The window has no inspector in production: what you write there goes into
 * the void, and it's in production that the defects we're looking for show
 * up. Lines therefore go to Rust, which writes them into ITS log — same
 * file, same clock, same order as its own.
 *
 * It's the only way to read a sequence that crosses two windows and a
 * process. Stitching two logs back together by hand requires knowing in
 * what order things happened, which is precisely the question being asked.
 *
 * ## It NEVER fails the caller
 *
 * Logging is a diagnostic operation: a lost line costs a line, an exception
 * thrown from a diagnostic `finally` costs the very action being observed.
 * Everything is swallowed.
 */
@Injectable({ providedIn: 'root' })
export class Log {
  private readonly bridge = inject(Bridge);

  /** A milestone in the sequence: what the window just did. */
  step(message: string): void {
    this.write('info', message);
  }

  /** A detail only read when someone is looking for it. */
  detail(message: string): void {
    this.write('debug', message);
  }

  /** Something unexpected, that doesn't prevent continuing. */
  concern(message: string): void {
    this.write('warn', message);
  }

  private write(level: Level, message: string): void {
    // Also to the console: in front of the dev server, that's where you
    // look, and the duplication costs nothing.
    console.info(`[${level}] ${message}`);
    void this.bridge.log(level, message).catch(() => {});
  }
}
