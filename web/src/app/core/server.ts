import { Injectable, inject, signal } from '@angular/core';

import type { ServerStatus } from './contracts';
import { Bridge } from './bridge';

/**
 * The server this pack declares, probed on request.
 *
 * ## Why this doesn't schedule its own refresh
 *
 * The service is a singleton, `providedIn: 'root'`; the thirty-second
 * cadence belongs to Spawn's `ngOnDestroy`, which is the only thing that
 * can promise a timer actually stops. A service-owned `setInterval` would
 * have nothing tying its lifetime to the one page that shows this — and
 * would outlive it. Same split as `Pack`: gestures and cadence live in the
 * component that owns a lifecycle, the service only holds the result and
 * knows how to ask for a fresh one.
 */
@Injectable({ providedIn: 'root' })
export class Server {
  private readonly bridge = inject(Bridge);

  /** `null` until the first probe answers. */
  readonly status = signal<ServerStatus | null>(null);

  /**
   * Asks again.
   *
   * Never rejects in practice — the command it calls turns an unreachable
   * server and a missing declaration into ordinary results, not errors —
   * but nothing here depends on that promise holding forever, and a caller
   * on a thirty-second timer needs to survive it not holding regardless.
   */
  async refresh(): Promise<void> {
    if (!this.bridge.available) {
      return;
    }
    this.status.set(await this.bridge.serverStatus());
  }
}
