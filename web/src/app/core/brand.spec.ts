import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Brand } from './brand';

describe('Brand', () => {
  let brand: Brand;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    brand = TestBed.inject(Brand);
  });

  /**
   * A default that is NOT "loading…".
   *
   * The title bar is visible from the very first frame: showing a waiting
   * text there would make the launcher's name flash on every open. The
   * default is therefore the likely name, replaced without it being
   * noticed.
   */
  it('shows a plausible name before having asked', () => {
    expect(brand.view().name).toBe('Helm');
    expect(brand.view().seal).toBe('HE');
  });

  /**
   * The seal fits in two characters: it's a badge, not a label. A longer
   * seal would overflow the title bar's square.
   */
  it('has a short seal', () => {
    expect(brand.view().seal.length).toBeLessThanOrEqual(2);
    expect(brand.view().name.length).toBeGreaterThan(0);
  });

  /**
   * Outside the Tauri window, there's no one to ask: the default stays, and
   * `load` doesn't reject. That's the case for `ng serve` and for this
   * suite.
   */
  it('outside Tauri, keeps its default without complaining', async () => {
    await expect(brand.load()).resolves.toBeUndefined();
    expect(brand.view().name).toBe('Helm');
  });
});
