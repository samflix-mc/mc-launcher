import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Boot } from './boot';
import { PHRASES, aPhrase } from './phrases';

/**
 * The boot screen, in the window.
 *
 * It carries the SAME drawing as Tauri's splash window and `index.html`'s
 * static boot: no transition between the three is visible, and there is
 * only one drawing to keep up to date.
 */
describe('Boot', () => {
  function mount(phrase?: string) {
    const fixture = TestBed.createComponent(Boot);
    if (phrase !== undefined) {
      fixture.componentRef.setInput('phrase', phrase);
    }
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
  });

  it('it carries the launcher name and a phrase', () => {
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="name"]').textContent.trim()).not.toBe(
      '',
    );
    expect(fixture.nativeElement.querySelector('[data-test="phrase"]').textContent.trim()).not.toBe(
      '',
    );
  });

  /** The caller can say something specific rather than a phrase at random. */
  it('a given phrase wins over the draw', () => {
    const fixture = mount('Checking the session…');

    expect(fixture.nativeElement.querySelector('[data-test="phrase"]').textContent).toContain(
      'Checking the session…',
    );
  });

  /**
   * **The bar is indeterminate**, and that's a claim we can back up: at
   * this point, the boot screen is waiting on the NETWORK, and no one knows
   * how long that takes. A bar climbing at a made-up pace would lie.
   */
  it('the bar doesn’t claim to know where things stand', () => {
    const fixture = mount();

    expect(
      fixture.nativeElement
        .querySelector('[data-test="progress"]')
        .classList.contains('hm-progress__fill--indeterminate'),
    ).toBe(true);
  });

  /**
   * It announces its wait to assistive technologies.
   *
   * The role is checked through the TAG and not through a `role` attribute:
   * `<output>` carries `status` implicitly, and writing it out again was
   * redundant. The assertion holds the same guarantee — the element is
   * announced — by naming what gives it rather than a duplicate of it.
   */
  it('it is announced as a status', () => {
    const boot = mount().nativeElement.querySelector('[data-test="boot"]');

    expect(boot.tagName).toBe('OUTPUT');
    expect(boot.getAttribute('aria-live')).toBe('polite');
  });
});

describe('aPhrase', () => {
  it('always renders a phrase from the list', () => {
    for (let attempt = 0; attempt < 50; attempt += 1) {
      expect(PHRASES).toContain(aPhrase());
    }
  });
});
