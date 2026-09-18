import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { LucideAngularModule } from 'lucide-angular';

import { Bell, Copy, Minus, Square, X } from '../../core/icons';
import { WindowService } from '../../core/window';
import { Brand } from '../../core/brand';
import { Notifications } from '../../core/notifications';
import { Pack } from '../../core/pack';

/**
 * The launcher's title bar, in place of the system one.
 *
 * ## What it carries, in the design system's order
 *
 * The launcher's name in pixel type, a separator, the name of the current
 * pack; then, pushed to the right, the notification bell and the three
 * window controls. The controls are forty-six pixels wide because that's the
 * width of Windows' caption buttons — a launcher that changes it gets
 * noticed, and never in a good way.
 *
 * ## What we did NOT have to write
 *
 * Edge resizing. `tauri-runtime-wry` wires up its own handler on the WebView
 * under Linux, with a five-pixel band multiplied by the scale factor. What
 * was missing wasn't the gesture but the CURSOR, and it's set by
 * `.hm-bords`, in the shell.
 *
 * ## No `pointer-events: none` on the children
 *
 * Tauri's drag script honors `data-tauri-drag-region` at any depth, and
 * itself excludes `A`, `BUTTON`, `INPUT`, `SELECT`, `TEXTAREA`, `LABEL`,
 * `SUMMARY`, any `[contenteditable]`, any non-negative `[tabindex]` and any
 * interactive `role`. Neutralizing pointer events would achieve nothing and
 * would cost us hover and cursor feedback.
 */
@Component({
  selector: 'app-title-bar',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './title-bar.html',
  styleUrl: './title-bar.css',
})
export class TitleBar {
  /**
   * The bar for a DIALOG window rather than the main one.
   *
   * It loses the maximize button — `--dialog` hides it — and the bell,
   * which has nothing to announce in a window where you do just one thing.
   * It gains the glass: the sign-in window has no image behind its bar,
   * only a frosted sheet, and a transparent bar would blend into it with no
   * way to tell where to grab it.
   *
   * Hiding the button isn't enough on the system side: it's
   * `maximizable: false` at window construction that stops a double-click
   * on the bar from maximizing it anyway.
   */
  readonly dialog = input(false);

  private readonly windowService = inject(WindowService);
  private readonly notifications = inject(Notifications);

  protected readonly brand = inject(Brand).view;
  protected readonly maximized = this.windowService.maximized;
  protected readonly unread = this.notifications.unread;
  protected readonly panelOpen = this.notifications.panelOpen;

  private readonly state = inject(Pack).state;

  /**
   * What follows the separator: the pack's name.
   *
   * The design system puts the server's name there. We only have one, and
   * what the player recognizes is the modpack's name — it's what the lock
   * publishes, and it's what changes when the server changes season.
   */
  protected readonly pack = computed(() => {
    // In a dialog window, what follows the separator is what we're DOING
    // there — the design system writes "Sign in". The modpack's name would
    // say nothing: we're not playing here, we're signing in.
    if (this.dialog()) {
      return 'Sign in';
    }
    return this.state()?.name ?? null;
  });

  // The icon nodes are passed to the template as values: a typo then becomes
  // a TypeScript error, whereas a registry resolved by name would render an
  // empty gap without a word.
  protected readonly Minus = Minus;
  protected readonly Square = Square;
  protected readonly Copy = Copy;
  protected readonly X = X;
  protected readonly Bell = Bell;

  protected switchNotifications(): void {
    this.notifications.togglePanel();
  }

  protected minimize(): void {
    void this.windowService.minimize();
  }

  protected switchMaximized(): void {
    void this.windowService.toggleMaximized();
  }

  protected close(): void {
    void this.windowService.close();
  }
}
