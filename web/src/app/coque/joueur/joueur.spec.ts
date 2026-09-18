import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { Session } from '../../noyau/session';
import { Joueur } from './joueur';

/**
 * Le badge du joueur, et le menu de son compte.
 *
 * Ce qu'il ne montre PAS compte autant que ce qu'il montre : ni UUID, ni
 * adresse, ni jeton — ce sont les trois choses qu'un joueur recopiera dans un
 * salon si l'interface les met sous ses yeux.
 */
describe('Joueur', () => {
  let session: Session;

  function monter() {
    const fixture = TestBed.createComponent(Joueur);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
    session = TestBed.inject(Session);
    session.compte.set({ pseudo: 'thesam1798', uuid: '0123', possedeLeJeu: true });
  });

  it('sans compte, il n’y a pas de badge', () => {
    session.compte.set(null);
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="badge-joueur"]')).toBeNull();
  });

  it('il montre le pseudo, et rien du compte Microsoft', () => {
    const fixture = monter();

    const badge = fixture.nativeElement.querySelector('[data-test="badge-joueur"]');
    expect(badge.textContent).toContain('thesam1798');
    expect(badge.textContent).not.toContain('0123');
  });

  /**
   * La tête est une image servie par un hôte distant. Hors ligne, elle casse —
   * et le badge afficherait alors le carré vide d'une image manquante. On
   * retombe sur le cube neutre du design system.
   */
  it('si la tête ne charge pas, le cube neutre tient la place', () => {
    const fixture = monter();

    const image = fixture.nativeElement.querySelector('[data-test="tete"]');
    image.dispatchEvent(new Event('error'));
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="tete"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="tete-absente"]')).not.toBeNull();
  });

  it('le menu ne s’ouvre qu’au clic, et se referme', () => {
    const fixture = monter();
    const badge = fixture.nativeElement.querySelector('[data-test="badge-joueur"]');

    expect(fixture.nativeElement.querySelector('[data-test="menu-compte"]')).toBeNull();

    badge.click();
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-test="menu-compte"]')).not.toBeNull();
    expect(badge.getAttribute('aria-expanded')).toBe('true');

    badge.click();
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-test="menu-compte"]')).toBeNull();
  });

  it('le menu porte le pseudo, rafraîchir et se déconnecter', () => {
    const fixture = monter();
    fixture.nativeElement.querySelector('[data-test="badge-joueur"]').click();
    fixture.detectChanges();

    const menu = fixture.nativeElement.querySelector('[data-test="menu-compte"]');
    expect(menu.textContent).toContain('thesam1798');
    expect(menu.querySelector('[data-test="rafraichir"]')).not.toBeNull();
    expect(menu.querySelector('[data-test="deconnexion"]')).not.toBeNull();
  });

  /**
   * Pendant qu'une opération de session tourne, les deux gestes du menu sont
   * refusés : les enchaîner ferait partir deux appels concurrents sur le même
   * jeton.
   */
  it('pendant une opération, le menu ne se clique pas', () => {
    session.occupe.set(true);
    const fixture = monter();
    fixture.nativeElement.querySelector('[data-test="badge-joueur"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="rafraichir"]').disabled).toBe(true);
    expect(fixture.nativeElement.querySelector('[data-test="deconnexion"]').disabled).toBe(true);
  });
});
