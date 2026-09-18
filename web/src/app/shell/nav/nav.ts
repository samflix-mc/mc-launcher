import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import { Compass, Newspaper, SlidersHorizontal } from '../../core/icons';

/** A section, and what to draw it with. */
export interface Tab {
  readonly path: string;
  readonly label: string;
  readonly icon: LucideIconData;
}

/**
 * The navigation pill, floating and centered at the top of the page.
 *
 * ## It replaces the left menu, and that's not a matter of taste
 *
 * The side menu took up a hundred and eighty pixels of full height for
 * three entries — that is, it ate a strip of the image across the whole
 * window, permanently. The design system puts the three sections in a
 * glass pill that only costs one line at the top: the middle of the screen
 * stays given over to the image, which is the whole point of this art
 * direction.
 *
 * ## Three entries, in this order, always visible
 *
 * Spawn is the landing page and navigation remembers nothing: reopening
 * the launcher reopens Spawn. A fourth section would first have to ask
 * itself whether it belongs to Settings instead.
 *
 * ## What's no longer here
 *
 * "Sign out" used to live in this bar; it's now in the player badge menu,
 * which is where you go when you're thinking about your account. And it
 * doesn't appear at all until there's a session: the pill itself doesn't
 * exist before sign-in.
 */
@Component({
  selector: 'app-nav',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule, RouterLink, RouterLinkActive],
  templateUrl: './nav.html',
  styleUrl: './nav.css',
})
export class Nav {
  protected readonly tabs: readonly Tab[] = [
    { path: '/spawn', label: 'Spawn', icon: Compass },
    { path: '/news', label: 'News', icon: Newspaper },
    { path: '/settings', label: 'Settings', icon: SlidersHorizontal },
  ];
}
