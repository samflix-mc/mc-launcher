import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { Nav } from './nav';

/**
 * The navigation pill.
 *
 * Three entries, in this order, always visible — that's a design system
 * rule, and a fourth section should first ask itself whether it belongs to
 * Settings instead.
 */
describe('Nav', () => {
  function mount() {
    const fixture = TestBed.createComponent(Nav);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
  });

  it('carries the three sections, in order', () => {
    const fixture = mount();

    const entries = [...fixture.nativeElement.querySelectorAll('[data-test^="tab-"]')];
    expect(entries.map((e: Element) => e.textContent?.trim())).toEqual([
      'Spawn',
      'News',
      'Settings',
    ]);
  });

  /**
   * ANCHORS and not buttons: they have a URL, and a URL can be copied, opened
   * in the middle, kept in history. The design system draws buttons; the
   * only thing to take back is removing the underline.
   */
  it('the entries are links to real routes', () => {
    const fixture = mount();

    const entries = [...fixture.nativeElement.querySelectorAll('[data-test^="tab-"]')];
    for (const entry of entries as HTMLElement[]) {
      expect(entry.tagName).toBe('A');
      expect(entry.getAttribute('href')).toMatch(/^\/(spawn|news|settings)$/);
    }
  });

  it('each entry is reachable by its test attribute', () => {
    const fixture = mount();

    for (const name of ['spawn', 'news', 'settings']) {
      expect(fixture.nativeElement.querySelector(`[data-test="tab-${name}"]`)).not.toBeNull();
    }
  });
});
