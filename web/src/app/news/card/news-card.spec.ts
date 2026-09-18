import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Post } from '../../core/contracts';
import { NewsCard } from './news-card';

function post(over: Partial<Post> = {}): Post {
  return {
    id: 'launch',
    title: 'The launcher is here',
    date: new Date().toISOString(),
    pinned: false,
    image: null,
    body: [
      { type: 'heading', level: 2, content: [{ type: 'text', text: 'A heading' }] },
      {
        type: 'paragraph',
        content: [
          { type: 'text', text: 'The launcher installs the pack ' },
          { type: 'bold', text: 'and launches the game' },
          { type: 'text', text: ' in a single gesture.' },
        ],
      },
    ],
    ...over,
  };
}

/**
 * A post's tile, in its three sizes.
 *
 * The design system describes them as modifiers of the same thing; these
 * tests keep the fact that they remain one single thing.
 */
describe('NewsCard', () => {
  function mount(entries: { post: Post; variant?: string; asLink?: boolean }) {
    const fixture = TestBed.createComponent(NewsCard);
    fixture.componentRef.setInput('post', entries.post);
    if (entries.variant) {
      fixture.componentRef.setInput('variant', entries.variant);
    }
    if (entries.asLink !== undefined) {
      fixture.componentRef.setInput('asLink', entries.asLink);
    }
    fixture.detectChanges();
    return fixture;
  }

  function tile(fixture: ReturnType<typeof mount>): HTMLElement {
    return fixture.nativeElement.querySelector('[data-test="news-card"]');
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
  });

  /**
   * **The gold trim is only carried by the pinned tile.**
   *
   * The design system allows only one per screen: it's the sole thing
   * highlighted, and two would therefore mean zero.
   */
  it('the pinned tile carries the gold trim, the others don’t', () => {
    const gold = tile(mount({ post: post(), variant: 'pinned' }));
    expect(gold.className).toContain('hm-glass--gold');
    expect(gold.className).toContain('hm-news--pinned');

    const plain = tile(mount({ post: post(), variant: 'tile' }));
    expect(plain.className).not.toContain('hm-glass--gold');
  });

  it('the featured tile takes its class', () => {
    expect(tile(mount({ post: post(), variant: 'featured' })).className).toContain(
      'hm-news--featured',
    );
  });

  /**
   * **The excerpt comes from the first PARAGRAPH**, and not from the body's
   * first characters: a heading at the top would give an excerpt that
   * looks like nothing.
   */
  it('the excerpt is the first paragraph, flattened', () => {
    const fixture = mount({ post: post(), variant: 'featured' });

    const excerpt = fixture.nativeElement.querySelector('[data-test="excerpt"]');
    expect(excerpt.textContent.trim()).toBe(
      'The launcher installs the pack and launches the game in a single gesture.',
    );
  });

  /** On a small tile, there's no room: the title is enough. */
  it('the small tile carries no excerpt', () => {
    const fixture = mount({ post: post(), variant: 'tile' });

    expect(fixture.nativeElement.querySelector('[data-test="excerpt"]')).toBeNull();
  });

  /** A post with no paragraph must not render an empty but present excerpt. */
  it('with no paragraph, no excerpt', () => {
    const fixture = mount({
      post: post({ body: [{ type: 'separator' }] }),
      variant: 'featured',
    });

    expect(fixture.nativeElement.querySelector('[data-test="excerpt"]')).toBeNull();
  });

  /**
   * **The illustration goes through a custom property.**
   *
   * Angular sanitizes the style values it binds, and a `url(data:…)` of
   * several hundred kilobytes is exactly the kind of value a sanitizer
   * replaces with nothing — without an error.
   */
  it('the illustration arrives through a custom property', () => {
    const fixture = mount({ post: post({ image: 'data:image/webp;base64,AAAA' }) });

    const media = fixture.nativeElement.querySelector('[data-test="media"]');
    expect(media.style.getPropertyValue('--illustration')).toContain('data:image/webp');
    expect(media.classList.contains('hm-news__media--ph')).toBe(false);
  });

  it('with no illustration, the replacement gradient holds the space', () => {
    const fixture = mount({ post: post() });

    const media = fixture.nativeElement.querySelector('[data-test="media"]');
    expect(media.classList.contains('hm-news__media--ph')).toBe(true);
  });

  /**
   * The tile is a LINK when it leads elsewhere, a BUTTON when it opens
   * here. A link to the page you're already on would do nothing at all.
   */
  it('it is a link, or a button that emits', () => {
    expect(tile(mount({ post: post() })).tagName).toBe('A');

    const fixture = mount({ post: post(), asLink: false });
    expect(tile(fixture).tagName).toBe('BUTTON');

    let opened: Post | null = null;
    fixture.componentInstance.open.subscribe((received) => (opened = received));
    tile(fixture).click();
    expect(opened).not.toBeNull();
  });

  it('a pinned post carries its label', () => {
    const fixture = mount({ post: post({ pinned: true }) });

    expect(fixture.nativeElement.querySelector('[data-test="pinned"]').textContent).toContain(
      'Pinned',
    );
  });
});
