import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Post, Feed } from '../core/contracts';
import { News } from '../core/news';
import { NewsPage } from './news';

function post(over: Partial<Post> = {}): Post {
  return {
    id: 'one',
    title: 'The launcher is here',
    date: new Date().toISOString(),
    pinned: false,
    image: null,
    body: [{ type: 'paragraph', content: [{ type: 'text', text: 'A post.' }] }],
    ...over,
  };
}

function feed(over: Partial<Feed> = {}): Feed {
  return { posts: [post()], offline: false, discarded: [], ...over };
}

/**
 * The news feed: a featured tile, then a grid.
 *
 * The assumed gap with the design system is the DESTINATION: there a tile
 * opens the server's site, here it opens a reading dialog — the body is
 * parsed by Rust into a typed tree, and that's the condition under which
 * the CSP was loosened.
 */
describe('NewsPage', () => {
  let news: News;

  function mount() {
    const fixture = TestBed.createComponent(NewsPage);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
    news = TestBed.inject(News);
  });

  it('an empty feed says so, without claiming a failure', () => {
    news.feed.set(feed({ posts: [] }));
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="no-news"]').textContent).toContain(
      'Nothing published',
    );
  });

  /**
   * **The featured post also stays in the grid.**
   *
   * That's the design system's rule: removing the highlighted post would
   * leave a hole in the timeline, and one would search a long time for why
   * it's missing.
   */
  it('the featured post stays in the grid', () => {
    news.feed.set(feed({ posts: [post({ id: 'a' }), post({ id: 'b' })] }));
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="featured"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelectorAll('[data-test="tile"]')).toHaveLength(2);
  });

  /**
   * A feed served from the cached copy must say so. A news page from
   * yesterday beats an empty page, but a page that lied about its
   * freshness would be worse than either.
   */
  it('an offline feed announces it', () => {
    news.feed.set(feed({ offline: true }));
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="offline"]').textContent).toContain(
      'offline',
    );
  });

  /** A half-faulty feed is noticeable, discreetly, for whoever publishes. */
  it('discarded posts are counted', () => {
    news.feed.set(feed({ discarded: ['unreadable date', 'missing title'] }));
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="discarded"]').textContent).toContain(
      '2',
    );
  });

  it('clicking a tile opens the post here, not elsewhere', () => {
    news.feed.set(feed());
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="reading"]')).toBeNull();

    fixture.nativeElement.querySelector('[data-test="featured"] button').click();
    fixture.detectChanges();

    const reading = fixture.nativeElement.querySelector('[data-test="reading"]');
    expect(reading).not.toBeNull();
    expect(reading.textContent).toContain('The launcher is here');
  });

  it('the reading dialog closes', () => {
    news.feed.set(feed());
    const fixture = mount();
    fixture.nativeElement.querySelector('[data-test="featured"] button').click();
    fixture.detectChanges();

    fixture.nativeElement.querySelector('[data-test="close-reading"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="reading"]')).toBeNull();
  });

  it('while loading, the grid keeps its shape', () => {
    news.loading.set(true);
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="loading"]')).not.toBeNull();
  });
});
