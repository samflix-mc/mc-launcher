import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Feed } from './contracts';
import { News } from './news';

function feed(posts: Feed['posts']): Feed {
  return { posts, offline: false, discarded: [] };
}

describe('News', () => {
  let news: News;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    news = TestBed.inject(News);
  });

  it('has nothing before having loaded', () => {
    expect(news.feed()).toBeNull();
    expect(news.latest()).toBeNull();
    expect(news.loading()).toBe(false);
  });

  /**
   * Spawn's card shows ONLY the first post. Sorting — pinned first, then
   * most recent to oldest — is done in Rust, so a mutation reaches it: here,
   * "the first one" is therefore indeed "the one to show", and this service
   * has nothing to reorder.
   */
  it('takes the first post for Spawn card', () => {
    news.feed.set(
      feed([
        {
          id: 'pinned',
          title: 'The most important',
          date: '2026-09-01T00:00:00Z',
          pinned: true,
          image: null,
          body: [],
        },
        {
          id: 'recent',
          title: 'The most recent',
          date: '2026-09-18T00:00:00Z',
          pinned: false,
          image: null,
          body: [],
        },
      ]),
    );

    expect(news.latest()?.id).toBe('pinned');
  });

  /**
   * An empty feed isn't an error — it's a host that doesn't publish news
   * yet, and the page must open on it without saying anything.
   */
  it('supports an empty feed', () => {
    news.feed.set(feed([]));

    expect(news.latest()).toBeNull();
    expect(news.feed()?.posts).toHaveLength(0);
  });

  /**
   * Outside Tauri, `load` requests nothing and doesn't reject. That's also
   * what keeps the page usable under `ng serve`.
   */
  it('outside Tauri, requests nothing', async () => {
    await expect(news.load()).resolves.toBeUndefined();

    expect(news.feed()).toBeNull();
    expect(news.loading()).toBe(false);
  });

  /**
   * The feed is kept for the session: coming back to the page doesn't
   * re-request it. A feed changes several times a month, not several times
   * a minute, and reloading it on every navigation would make the page
   * flash.
   */
  it('does not reload a feed already there', async () => {
    const already = feed([]);
    news.feed.set(already);

    await news.load();

    expect(news.feed()).toBe(already);
  });
});
