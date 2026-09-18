import { Injectable, inject, signal } from '@angular/core';

import type { Brand as BrandView } from './contracts';
import { Bridge } from './bridge';

/**
 * Under what name the launcher presents itself.
 *
 * ## Why a dedicated service
 *
 * The name belongs to neither the session, the pack, nor incidents — and TWO
 * consumers read it: the title bar, which lives in the shell, and Spawn's
 * banner. The title bar can't ask Spawn for it: it's above it in the tree,
 * and it exists even when Spawn isn't the page shown.
 *
 * It's requested ONCE, on first access, and kept: it's fixed at compile time
 * on the Rust side, so it won't change during the session.
 */
@Injectable({ providedIn: 'root' })
export class Brand {
  private readonly bridge = inject(Bridge);

  /**
   * A default that isn't "loading…".
   *
   * The title bar is visible from the very first frame: showing a waiting
   * text there would make the launcher's name flash on every open. The
   * default is therefore the likely name, replaced without it being noticed.
   */
  readonly view = signal<BrandView>({ name: 'Helm', seal: 'HE' });

  private requested = false;

  /** Requests the brand from Rust, once per session. */
  async load(): Promise<void> {
    if (this.requested || !this.bridge.available) {
      return;
    }
    this.requested = true;
    try {
      this.view.set(await this.bridge.brand());
    } catch {
      // The default stays shown. A brand that couldn't be read doesn't
      // prevent playing, and an error banner for a window name would be out
      // of proportion.
      this.requested = false;
    }
  }
}
