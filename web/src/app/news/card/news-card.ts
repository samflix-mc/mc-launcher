import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LucideAngularModule } from 'lucide-angular';

import { ChevronRight } from '../../core/icons';
import type { Post } from '../../core/contracts';
import { sayTheDate } from '../../core/dates';

/** The design system's three tile sizes. */
export type Variant = 'pinned' | 'featured' | 'tile';

/**
 * A post, as a TILE: the image fills it, a gradient rises from the bottom,
 * the text sits on top.
 *
 * ## Three sizes, one single component
 *
 * `pinned` on Spawn — the tall tile with the gold trim, one per screen;
 * `featured` at the top of News, full width; `tile` in the grid. The design
 * system describes them as modifiers of the same thing, and splitting them
 * into three components would make the same gradient diverge three times.
 *
 * ## It opens the page, not the browser
 *
 * That's the assumed gap with the design system. There, a post lives on the
 * server's site and the tile links to it; here, the body is parsed by Rust
 * into a typed tree and rendered INSIDE the window — that's the condition
 * under which the CSP was loosened, and there's no URL to leave to. The
 * chevron therefore says "open here" and not "leave", which is the only
 * honest thing to promise.
 */
@Component({
  selector: 'app-news-card',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink, LucideAngularModule, NgTemplateOutlet],
  templateUrl: './news-card.html',
  styleUrl: './news-card.css',
})
export class NewsCard {
  readonly post = input.required<Post>();
  readonly variant = input<Variant>('tile');

  /**
   * Does the tile lead to the news page, or does it open here?
   *
   * On Spawn, it leads: there's nothing to read in place. On the news page,
   * it opens the post in a dialog — putting a link there to the page you're
   * already on would do nothing at all.
   */
  readonly asLink = input(true);

  /** Emitted when the tile isn't a link and gets clicked. */
  readonly open = output<Post>();

  protected readonly ChevronRight = ChevronRight;

  protected readonly when = computed(() => sayTheDate(this.post().date));

  /** The tile's classes, based on its size. */
  protected readonly classes = computed(() => {
    const base = 'hm-glass hm-glass--interactive hm-news';
    switch (this.variant()) {
      case 'pinned':
        // The gold trim: the design system allows only ONE per screen, and
        // this is it — the sole thing highlighted on Spawn.
        return `${base} hm-news--pinned hm-glass--gold`;
      case 'featured':
        return `${base} hm-news--featured`;
      case 'tile':
        return base;
    }
  });

  /** The excerpt only displays on the two larger sizes. */
  protected readonly showExcerpt = computed(() => this.variant() !== 'tile');

  /**
   * The first paragraph, flattened to text.
   *
   * The first PARAGRAPH and not the body's first characters: a heading or a
   * list at the top would give an excerpt that looks like nothing.
   */
  protected readonly excerpt = computed(() => {
    const first = this.post().body.find((block) => block.type === 'paragraph');
    if (first?.type !== 'paragraph') {
      return '';
    }
    return first.content
      .map((piece) => piece.text)
      .join('')
      .trim();
  });
}
