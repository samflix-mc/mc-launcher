import { Injectable, computed, inject, signal } from '@angular/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

import type { DeviceCode, Account } from './contracts';
import { Bridge } from './bridge';

/**
 * Who is signed in, and how you become that.
 *
 * ## The single predicate
 *
 * [`playable`] is THE predicate both router guards depend on. A single
 * one, and that's deliberate: two different predicates would make an
 * account connected WITHOUT A LICENSE bounce forever between `/signin` and
 * `/spawn` — one would find it signed in, the other not playable, and each
 * would redirect to the other.
 *
 * This case isn't theoretical: it's a valid Microsoft account that never
 * bought Minecraft, and the screen has to say so rather than loop.
 */
@Injectable({ providedIn: 'root' })
export class Session {
  private readonly bridge = inject(Bridge);

  /** The account, or `null`. `undefined` until we've asked yet. */
  readonly account = signal<Account | null | undefined>(undefined);

  /** The code to enter on Microsoft's side, during the wait. */
  readonly code = signal<DeviceCode | null>(null);

  /** Is a call in progress? */
  readonly busy = signal(false);

  /** Have we already asked Rust? */
  readonly known = computed(() => this.account() !== undefined);

  /** Signed in AND owns the game. THE predicate for both guards. */
  readonly playable = computed(() => this.account()?.ownsTheGame === true);

  /** Signed in, but without a license: a state to display, not to redirect on. */
  readonly noLicense = computed(() => this.account()?.ownsTheGame === false);

  private unsubscribe: UnlistenFn | null = null;

  /**
   * Asks Rust, once.
   *
   * The access lazily refreshes tokens on Rust's side: without this call,
   * the next launch would start from the expired token and ask for a code
   * for nothing.
   */
  async open(): Promise<void> {
    if (!this.bridge.available) {
      this.account.set(null);
      return;
    }
    this.unsubscribe ??= await this.bridge.onDeviceCode((code) => this.code.set(code));
    this.account.set(await this.bridge.status());
  }

  /**
   * Opens a Microsoft session.
   *
   * Only returns once the player has gone through Microsoft — or the code
   * has expired. The code itself arrived as an event during the wait.
   */
  async signIn(): Promise<void> {
    this.busy.set(true);
    try {
      this.account.set(await this.bridge.signIn());
      this.code.set(null);
    } finally {
      this.busy.set(false);
    }
  }

  async signOut(): Promise<void> {
    this.busy.set(true);
    try {
      await this.bridge.signOut();
      this.account.set(null);
      this.code.set(null);
    } finally {
      this.busy.set(false);
    }
  }
}
