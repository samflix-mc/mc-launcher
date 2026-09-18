import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import { CircleCheck, CircleX, Download, Info, Trash2, TriangleAlert, X } from '../../core/icons';
import { sayTheDate } from '../../core/dates';
import { Notifications as Service, type Tone } from '../../core/notifications';

/**
 * The design system's two visible levels: toasts, and the center.
 *
 * ## One single component for both, and that's deliberate
 *
 * They share the same vocabulary — the same tone, the same icon, the same
 * title — and every toast is also written to the center. Splitting them
 * into two components would force duplicating the tone → icon mapping,
 * which would drift at the first addition: a gold toast and a gray line for
 * the same event.
 *
 * ## They live at the WINDOW level
 *
 * `.hm-toasts` sits bottom left, `.hm-notif` under the bell: both are
 * positioned relative to the window. Mounting this component in the page
 * would have it bounded by the first ancestor carrying a `backdrop-filter` —
 * the navigation pill, for instance, which creates one.
 */
@Component({
  selector: 'app-notifications',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './notifications.html',
  styleUrl: './notifications.css',
})
export class Notifications {
  private readonly service = inject(Service);

  protected readonly toasts = this.service.toasts;
  protected readonly log = this.service.log;
  protected readonly panelOpen = this.service.panelOpen;

  protected readonly X = X;
  protected readonly Trash2 = Trash2;

  /**
   * The icon for a tone.
   *
   * Color alone wouldn't be enough: the design system forbids it explicitly —
   * "success and danger are only told apart by hue" — and an icon is what
   * makes the difference for someone who tells them apart poorly.
   */
  protected icon(tone: Tone): LucideIconData {
    switch (tone) {
      case 'success':
        return CircleCheck;
      case 'danger':
        return CircleX;
      case 'warning':
        return TriangleAlert;
      case 'progress':
        return Download;
      case 'info':
        return Info;
    }
  }

  protected when(iso: string): string {
    return sayTheDate(iso);
  }

  protected close(id: number): void {
    this.service.closeToast(id);
  }

  protected closePanel(): void {
    this.service.closePanel();
  }

  protected clear(): void {
    this.service.empty();
  }
}
