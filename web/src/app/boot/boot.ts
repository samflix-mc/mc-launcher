import { ChangeDetectionStrategy, Component, inject, input } from '@angular/core';

import { Brand } from '../core/brand';
import { aPhrase } from './phrases';

/**
 * The boot screen, in the window — the same drawing as Tauri's.
 *
 * ## The three screens are identical, and that's the whole point
 *
 * There are three of them in a row: Tauri's splash window
 * (`public/splash.html`), `index.html`'s static boot, and this one. All
 * three carry `.hm-splash`, `.hm-wordmark` and the same progress bar: no
 * transition is visible, and there is only one drawing to keep up to date.
 *
 * ## What it must NOT cover
 *
 * The title bar. `status()` chains two network round trips, and on a slow
 * network or behind a captive portal, that takes a while. A full-screen boot
 * would then leave a window with no system button — since they were
 * removed — and no app button — since they'd be hidden underneath.
 *
 * ## The phrase is drawn ONCE
 *
 * In a field, not in an accessor: an accessor would redraw it on every
 * change detection, and it would change several times a second — the
 * opposite of what you want from a text meant to be read.
 */
@Component({
  selector: 'app-boot',
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './boot.html',
  styleUrl: './boot.css',
})
export class Boot {
  /**
   * What the boot screen announces, if the caller has something specific to
   * say.
   *
   * Otherwise, a phrase drawn at random.
   */
  readonly phrase = input<string | null>(null);

  protected readonly brand = inject(Brand).view;

  /** Drawn once, at construction. */
  protected readonly defaultPhrase = aPhrase();
}
