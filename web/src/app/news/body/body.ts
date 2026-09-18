import { NgTemplateOutlet } from '@angular/common';
import { LucideAngularModule } from 'lucide-angular';
import { ChangeDetectionStrategy, Component, inject, input } from '@angular/core';

import type { Block, Inline } from '../../core/contracts';
import { ExternalLink } from '../../core/icons';
import { Incidents } from '../../core/incidents';
import { Bridge } from '../../core/bridge';

/**
 * A post's body, rendered from the typed tree.
 *
 * ## Zero `innerHTML`, and that's this component's reason for existing
 *
 * The launcher's CSP loosens `style-src` up to `'unsafe-inline'`. That
 * loosening holds on one single condition: no markup comes from anywhere
 * other than the Angular compiler.
 *
 * The markdown is therefore parsed IN RUST into closed variants, and this
 * component walks them with `@switch`. It never receives a string it would
 * have to interpret — only text to display and structures to walk.
 * `pnpm invariants` checks that no `innerHTML` lingers; this component is
 * what makes that invariant tenable.
 *
 * ## Links leave through the system
 *
 * No `href` is ever set on an `<a>`: a click would navigate inside the
 * window, which Rust's navigation plugin would refuse — leaving the player
 * in front of a link that does nothing. They go through `openPage()`,
 * which hands them off to the system's browser.
 */
@Component({
  selector: 'app-post-body',
  changeDetection: ChangeDetectionStrategy.OnPush,
  // The component's only import: the fragments template is written once and
  // reused by the three blocks that contain some. Copying it would give
  // three chances to forget a case — and a forgotten case renders blank,
  // not an error.
  imports: [LucideAngularModule, NgTemplateOutlet],
  templateUrl: './body.html',
  styleUrl: './body.css',
})
export class PostBody {
  readonly blocks = input.required<readonly Block[]>();

  private readonly bridge = inject(Bridge);
  private readonly incidents = inject(Incidents);

  protected readonly ExternalLink = ExternalLink;

  protected async open(inline: Inline): Promise<void> {
    if (inline.type !== 'link') {
      return;
    }
    await this.incidents.guard(() => this.bridge.openPage(inline.href));
  }
}
