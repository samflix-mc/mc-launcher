import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Settings } from './settings';
import { SettingsService } from '../core/settings';

/**
 * The settings page.
 *
 * Two things get tested here, and they're the two that were flagged during
 * acceptance: that the sliders follow the finger, and that nothing goes over
 * the network before release.
 */
describe('Settings', () => {
  let settings: SettingsService;

  function mount() {
    const fixture = TestBed.createComponent(Settings);
    fixture.detectChanges();
    return fixture;
  }

  function slider(fixture: ReturnType<typeof mount>, test: string): HTMLInputElement {
    return fixture.nativeElement.querySelector(`[data-test="${test}"]`);
  }

  function read(fixture: ReturnType<typeof mount>, test: string): string | null {
    const element = fixture.nativeElement.querySelector(`[data-test="${test}"]`);
    return element ? element.textContent.replace(/\s+/g, ' ').trim() : null;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
    settings = TestBed.inject(SettingsService);
  });

  /**
   * **The slider's default follows the draft, and the draft follows the
   * service.** Without this chain, the page would open on the original
   * values and overwrite them on the first gesture.
   */
  it('the controls display what the service holds', () => {
    const fixture = mount();

    expect(slider(fixture, 'render').value).toBe(String(settings.view().game.renderDistance));
    expect(read(fixture, 'scrim-value')).toContain(
      `${(settings.view().appearance.scrim * 100).toFixed(0)} %`,
    );
  });

  /**
   * **The displayed number follows the finger.**
   *
   * This is the visible half of the bug that was reported: "it moves the
   * handle, but it doesn't change the value". The local draft is what fixes
   * it.
   */
  it('during the gesture, the displayed value moves', () => {
    const fixture = mount();
    const render = slider(fixture, 'render');

    render.value = '24';
    render.dispatchEvent(new Event('input'));
    fixture.detectChanges();

    expect(read(fixture, 'render-value')).toBe('24');
  });

  /**
   * **And nothing is written while dragging.**
   *
   * The other half: the previous version saved on `input`, i.e. thirty to
   * fifty writes per second whose responses came back out of order — the
   * oldest overwriting the most recent.
   */
  it('during the gesture, nothing is written', () => {
    const fixture = mount();
    const render = slider(fixture, 'render');
    const before = settings.view().game.renderDistance;

    for (const value of ['14', '18', '22', '26']) {
      render.value = value;
      render.dispatchEvent(new Event('input'));
    }
    fixture.detectChanges();

    expect(settings.view().game.renderDistance).toBe(before);
  });

  /** On release, a single write — and it carries the last value. */
  it('on release, the value is saved', async () => {
    const fixture = mount();
    const render = slider(fixture, 'render');

    render.value = '26';
    render.dispatchEvent(new Event('input'));
    render.dispatchEvent(new Event('change'));
    await fixture.whenStable();

    expect(settings.view().game.renderDistance).toBe(26);
  });

  /**
   * **The scrim can go all the way down to zero.**
   *
   * It used to be bounded at 0.44 back when it was the one holding the
   * contrast. That's no longer the case — the glass thickens on its own —
   * and a slider still refusing to go there would render a gesture that
   * jumps.
   */
  it('the scrim can go all the way to zero', () => {
    const fixture = mount();

    expect(slider(fixture, 'scrim').min).toBe('0');
  });

  /**
   * Memory goes up to sixty-four gigabytes. It used to stop at thirty-two,
   * which cut the slider in half on Sam's machine without anything saying
   * why.
   */
  it('memory goes up to sixty-four gigabytes', () => {
    const fixture = mount();

    expect(slider(fixture, 'memory').max).toBe('64');
  });

  /**
   * **The rail puts nothing in the URL.**
   *
   * Its entries used to be anchors; with the fragment, the router mistook
   * them for routes. They're buttons, and nothing in their markup should
   * become a link again.
   */
  it('the rail is made of buttons, with no href', () => {
    const fixture = mount();

    const entries = [...fixture.nativeElement.querySelectorAll('[data-test^="rail-"]')];
    expect(entries).toHaveLength(5);
    for (const entry of entries as HTMLElement[]) {
      expect(entry.tagName).toBe('BUTTON');
      expect(entry.getAttribute('href')).toBeNull();
    }
  });

  /**
   * A single verification, and it's the full one. The "quick" one only
   * compared sizes: it said "everything is in place" about a corrupted file
   * of the right length, i.e. the one case worth verifying.
   */
  it('there is only one verify button', () => {
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="verify"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="verify-quick"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="verify-deep"]')).toBeNull();
  });
});
