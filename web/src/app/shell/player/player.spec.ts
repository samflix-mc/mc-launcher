import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { Session } from '../../core/session';
import { Player } from './player';

/**
 * The player badge, and their account menu.
 *
 * What it does NOT show matters as much as what it shows: no UUID, no
 * address, no token — those are the three things a player would paste into
 * a channel if the interface put them in view.
 */
describe('Player', () => {
  let session: Session;

  function mount() {
    const fixture = TestBed.createComponent(Player);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
    session = TestBed.inject(Session);
    session.account.set({ username: 'thesam1798', uuid: '0123', ownsTheGame: true });
  });

  it('with no account, there is no badge', () => {
    session.account.set(null);
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="player-badge"]')).toBeNull();
  });

  it('it shows the username, and nothing of the Microsoft account', () => {
    const fixture = mount();

    const badge = fixture.nativeElement.querySelector('[data-test="player-badge"]');
    expect(badge.textContent).toContain('thesam1798');
    expect(badge.textContent).not.toContain('0123');
  });

  /**
   * The head is an image served by a remote host. Offline, it breaks — and
   * the badge would then show the empty square of a missing image. We fall
   * back to the design system's neutral cube.
   */
  it('if the head fails to load, the neutral cube holds the place', () => {
    const fixture = mount();

    const image = fixture.nativeElement.querySelector('[data-test="head"]');
    image.dispatchEvent(new Event('error'));
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="head"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="head-missing"]')).not.toBeNull();
  });

  it('the menu only opens on click, and closes', () => {
    const fixture = mount();
    const badge = fixture.nativeElement.querySelector('[data-test="player-badge"]');

    expect(fixture.nativeElement.querySelector('[data-test="account-menu"]')).toBeNull();

    badge.click();
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-test="account-menu"]')).not.toBeNull();
    expect(badge.getAttribute('aria-expanded')).toBe('true');

    badge.click();
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-test="account-menu"]')).toBeNull();
  });

  it('the menu carries the username, refresh, and sign out', () => {
    const fixture = mount();
    fixture.nativeElement.querySelector('[data-test="player-badge"]').click();
    fixture.detectChanges();

    const menu = fixture.nativeElement.querySelector('[data-test="account-menu"]');
    expect(menu.textContent).toContain('thesam1798');
    expect(menu.querySelector('[data-test="refresh"]')).not.toBeNull();
    expect(menu.querySelector('[data-test="sign-out"]')).not.toBeNull();
  });

  /**
   * While a session operation is running, both menu actions are refused:
   * chaining them would fire two concurrent calls on the same token.
   */
  it('while an operation is running, the menu is not clickable', () => {
    session.busy.set(true);
    const fixture = mount();
    fixture.nativeElement.querySelector('[data-test="player-badge"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="refresh"]').disabled).toBe(true);
    expect(fixture.nativeElement.querySelector('[data-test="sign-out"]').disabled).toBe(true);
  });
});
