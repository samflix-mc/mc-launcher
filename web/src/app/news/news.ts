import {
  ChangeDetectionStrategy,
  Component,
  type OnInit,
  computed,
  inject,
  signal,
} from '@angular/core';
import { LucideAngularModule } from 'lucide-angular';

import { Globe, RefreshCw, X } from '../core/icons';
import type { Post } from '../core/contracts';
import { sayTheDate } from '../core/dates';
import { Incidents } from '../core/incidents';
import { News } from '../core/news';
import { NewsCard } from './card/news-card';
import { PostBody } from './body/body';

/**
 * The news feed: a featured tile, then a grid.
 *
 * The class name is `NewsPage` and not `News`: the service is already
 * called that, and two symbols with the same name in the same file would
 * force renaming one at import — which reads badly.
 *
 * ## The assumed gap with the design system
 *
 * There, a tile opens the post on the server's SITE, in the browser, and the
 * launcher never displays a body. Here, the body is parsed by Rust into a
 * typed tree and rendered in the window — that's the condition under which
 * the CSP was loosened, and it's what lets this repo have zero `innerHTML`.
 *
 * The tile therefore opens a reading dialog. What's kept from the design
 * system is the shape: the featured tile, the grid, the gradient, the
 * chevron. What isn't is the destination — and it was better to change it
 * than to lie about what the tile does.
 */
@Component({
  selector: 'app-news',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [NewsCard, PostBody, LucideAngularModule],
  templateUrl: './news.html',
  styleUrl: './news.css',
})
export class NewsPage implements OnInit {
  private readonly news = inject(News);
  private readonly incidents = inject(Incidents);

  protected readonly feed = this.news.feed;
  protected readonly loading = this.news.loading;
  protected readonly featured = this.news.pinned;

  /** The post open for reading, or `null`. */
  protected readonly reading = signal<Post | null>(null);

  protected readonly RefreshCw = RefreshCw;
  protected readonly Globe = Globe;
  protected readonly X = X;

  /**
   * The grid: the whole feed, featured post included.
   *
   * The design system says so: "the featured tile also appears in the grid
   * in date order, so the list stays complete". Removing the highlighted
   * post would leave a hole in the timeline, and one would search a long
   * time for why it's missing.
   */
  protected readonly grid = computed(() => this.feed()?.posts ?? []);

  ngOnInit(): void {
    void this.incidents.guard(() => this.news.load());
  }

  protected async reload(): Promise<void> {
    await this.incidents.guard(() => this.news.reload());
  }

  protected when(iso: string): string {
    return sayTheDate(iso);
  }
}
