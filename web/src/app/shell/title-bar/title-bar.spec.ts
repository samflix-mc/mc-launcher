import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Brand } from '../../core/brand';
import { Notifications } from '../../core/notifications';
import { Pack } from '../../core/pack';
import { TitleBar } from './title-bar';

/**
 * The title bar, in its two forms.
 *
 * The main window's carries the modpack's name, the bell and the three
 * controls; a dialog window's loses the bell and the maximize button.
 */
describe('TitleBar', () => {
  function mount(dialog = false) {
    const fixture = TestBed.createComponent(TitleBar);
    fixture.componentRef.setInput('dialog', dialog);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
  });

  it('it carries the launcher name', () => {
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="title"]').textContent).toContain(
      TestBed.inject(Brand).view().name,
    );
  });

  /**
   * What follows the separator is the MODPACK's name — what the player
   * recognizes, and what changes when the server changes season. The design
   * system puts the server's name there; we only have one.
   */
  it('the pack name follows the separator, when known', () => {
    expect(mount().nativeElement.querySelector('[data-test="pack"]')).toBeNull();

    TestBed.inject(Pack).state.set({
      action: 'play',
      drift: 'up-to-date',
      offline: false,
      installed: true,
      name: 'samflix',
      version: '1.4.2',
      java: 21,
      mods: 128,
      generation: 1,
    });
    expect(mount().nativeElement.querySelector('[data-test="pack"]').textContent).toContain(
      'samflix',
    );
  });

  /** The badge is hidden at zero: "0" would demand to be read for nothing. */
  it('the bell only counts what is unread', () => {
    expect(mount().nativeElement.querySelector('[data-test="unread"]')).toBeNull();

    TestBed.inject(Notifications).notify('info', 'One');
    expect(mount().nativeElement.querySelector('[data-test="unread"]').textContent).toContain('1');
  });

  it('the three window controls are there', () => {
    const fixture = mount();

    for (const name of ['minimize', 'maximize', 'close']) {
      expect(fixture.nativeElement.querySelector(`[data-test="${name}"]`)).not.toBeNull();
    }
  });

  /**
   * **In a dialog window, the bell leaves and the title changes.**
   *
   * It has nothing to announce where you're doing just one thing, and what
   * follows the separator is what we're DOING there — the modpack's name
   * would say nothing.
   */
  it('in a dialog, no bell, and the title says what we do here', () => {
    const fixture = mount(true);

    expect(fixture.nativeElement.querySelector('[data-test="bell"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="pack"]').textContent).toContain(
      'Sign in',
    );
  });

  /**
   * `--dialog` hides the maximize button, and `--glass` gives the bar
   * something to be seen against on a frosted sheet with no image behind it.
   */
  it('in a dialog, the bar takes both its modifiers', () => {
    const bar = mount(true).nativeElement.querySelector('[data-test="title-bar"]');

    expect(bar.className).toContain('hm-titlebar--dialog');
    expect(bar.className).toContain('hm-titlebar--glass');
  });
});
