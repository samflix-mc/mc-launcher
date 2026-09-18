import { Injectable, computed, inject, signal } from '@angular/core';

import type { Feed } from './contracts';
import { Bridge } from './bridge';

/**
 * The network's news feed.
 *
 * ## What this service does NOT do
 *
 * It sorts nothing, filters nothing, parses nothing. All of that is in
 * Rust — so a mutation reaches it, and because markdown becomes a typed tree
 * there rather than HTML. This service keeps a result and says whether one
 * is currently coming in.
 *
 * ## The feed is kept for the session
 *
 * Coming back to the page doesn't re-request it. A news feed changes several
 * times a month, not several times a minute, and reloading it on every
 * navigation would make the page flash for nothing. `reload()` exists for
 * the explicit gesture.
 */
@Injectable({ providedIn: 'root' })
export class News {
  private readonly bridge = inject(Bridge);

  readonly feed = signal<Feed | null>(null);
  readonly loading = signal(false);

  /** The most recent post, for Spawn's card. */
  readonly latest = computed(() => this.feed()?.posts[0] ?? null);

  /**
   * The featured post: the pinned one if there is one, otherwise the most
   * recent.
   *
   * That's the design system's rule, and it has a reason: the featured tile
   * is the only place where whoever publishes can push something ahead of
   * the timeline. Without pinning, it just settles for being the most
   * recent, which is already what the feed says.
   *
   * Rust already sorts — pinned first, then by descending date — so the
   * first element is enough. We say it again here rather than relying on it
   * silently: a feed whose sort order changed would make this page wrong
   * without any test complaining.
   */
  readonly pinned = computed(() => {
    const posts = this.feed()?.posts ?? [];
    return posts.find((post) => post.pinned) ?? posts[0] ?? null;
  });

  /** Loads the feed, once per session. */
  async load(): Promise<void> {
    if (this.feed() || this.loading() || !this.bridge.available) {
      return;
    }
    await this.reload();
  }

  /**
   * Re-requests the feed, even if it's already there.
   *
   * The error BUBBLES UP rather than being swallowed: it's up to the caller
   * to decide whether it deserves an overlay — on the news page, yes; on
   * Spawn's card, no, where it just leaves the card empty.
   */
  async reload(): Promise<void> {
    this.loading.set(true);
    try {
      this.feed.set(await this.bridge.news());
    } finally {
      this.loading.set(false);
    }
  }
}
