import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { LucideAngularModule } from 'lucide-angular';

import { Copy, TriangleAlert } from '../../core/icons';
import { Incidents } from '../../core/incidents';

/**
 * What went wrong, above everything else.
 *
 * ## The shape comes from the design system, and it has a rule
 *
 * What happened, then what to do about it, in two short sentences. The
 * technical detail — the message Rust returned — is what we ask to be
 * copied: it's selectable, in the mono face, and it carries the button that
 * copies it.
 *
 * ## It lives at the WINDOW level, and that's a constraint
 *
 * The navigation pill and the panels carry a `backdrop-filter`, which
 * creates a stacking context and makes them a containing block for their
 * positioned descendants. A dialog mounted inside one would be bounded to
 * its width, and would leave the rest of the screen clickable.
 *
 * ## No blur on the content
 *
 * The old version blurred the content area. The design system dims instead:
 * `.hm-scrim` darkens, which costs a composite instead of a blur — and
 * stacking two blurs is expensive on a WebView without DMA-BUF, which is
 * exactly the NVIDIA setup this launcher runs into.
 */
@Component({
  selector: 'app-incident',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './incident.html',
  styleUrl: './incident.css',
})
export class Incident {
  private readonly incidents = inject(Incidents);

  protected readonly message = this.incidents.current;

  protected readonly TriangleAlert = TriangleAlert;
  protected readonly Copy = Copy;

  protected close(): void {
    this.incidents.close();
  }

  protected async copy(): Promise<void> {
    const text = this.message();
    if (!text) {
      return;
    }
    // Without a `catch`, a refused clipboard would leave a promise rejected
    // into the void — and the original error would be replaced in the
    // window by the copy's, which is doubly useless.
    await navigator.clipboard.writeText(text).catch(() => {});
  }
}
