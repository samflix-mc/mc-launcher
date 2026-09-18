import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { Session } from '../noyau/session';
import { Connexion } from './connexion';

/**
 * La page de connexion, dans ses trois états plus un.
 *
 * Le quatrième n'est pas un pas : un compte Microsoft valide qui n'a jamais
 * acheté Minecraft. Les deux gardes du routeur lisent le MÊME prédicat pour que
 * ce cas ne rebondisse pas entre elles ; il reste ici, et l'écran le dit.
 */
describe('Connexion', () => {
  let session: Session;

  function monter() {
    const fixture = TestBed.createComponent(Connexion);
    fixture.detectChanges();
    return fixture;
  }

  function lire(fixture: ReturnType<typeof monter>, test: string): string | null {
    const element = fixture.nativeElement.querySelector(`[data-test="${test}"]`);
    return element ? element.textContent.replace(/\s+/g, ' ').trim() : null;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
    session = TestBed.inject(Session);
  });

  it('au départ, elle invite à se connecter', () => {
    const fixture = monter();

    expect(lire(fixture, 'connecter')).toContain('Se connecter avec Microsoft');
    expect(fixture.nativeElement.querySelector('[data-test="code"]')).toBeNull();
  });

  /**
   * Les trois pas sont la « barre de chargement » demandée à la recette : ils
   * disent où l'on en est d'une suite qui dure, au lieu de laisser croire que
   * rien ne bouge.
   */
  it('le premier pas est franchi dès l’ouverture', () => {
    const fixture = monter();

    const pas = [...fixture.nativeElement.querySelector('[data-test="pas"]').children];
    expect(pas.length).toBe(3);
    expect(pas.filter((p: Element) => p.classList.contains('hm-auth__step--done')).length).toBe(1);
  });

  it('le code arrivé, elle le montre et annonce l’attente', () => {
    session.code.set({
      code: 'FKRD-QXZB',
      url: 'https://www.microsoft.com/link',
      urlDirecte: 'https://www.microsoft.com/link?otc=FKRD-QXZB',
    });
    const fixture = monter();

    expect(lire(fixture, 'code')).toBe('FKRD-QXZB');
    expect(lire(fixture, 'attente')).toContain('En attente de Microsoft');
    expect(lire(fixture, 'rouvrir')).toContain('microsoft.com/link');
  });

  it('au pas du code, deux des trois pas sont franchis', () => {
    session.code.set({ code: 'A', url: 'https://x', urlDirecte: 'https://x' });
    const fixture = monter();

    const pas = [...fixture.nativeElement.querySelector('[data-test="pas"]').children];
    expect(pas.filter((p: Element) => p.classList.contains('hm-auth__step--done')).length).toBe(2);
  });

  /**
   * Le code est ce qu'on demande de recopier : il doit pouvoir être
   * sélectionné, là où le reste de la fenêtre ne le permet pas.
   */
  it('le code est sélectionnable, et copiable', () => {
    session.code.set({ code: 'A', url: 'https://x', urlDirecte: 'https://x' });
    const fixture = monter();

    expect(
      fixture.nativeElement
        .querySelector('[data-test="puits-code"]')
        .classList.contains('hm-selectionnable'),
    ).toBe(true);
    expect(fixture.nativeElement.querySelector('[data-test="copier-code"]')).not.toBeNull();
  });

  /**
   * **« Connecté mais sans licence » est un ÉTAT, pas une redirection.**
   *
   * Le dire coûte trois lignes ; ne pas le dire coûte huit cents mégaoctets
   * téléchargés pour apprendre ensuite qu'aucun serveur n'acceptera le compte.
   */
  it('un compte sans licence est un état, et il le dit', () => {
    session.compte.set({ pseudo: 'thesam1798', uuid: '0123', possedeLeJeu: false });
    const fixture = monter();

    expect(lire(fixture, 'sans-licence')).toContain('ne possède pas Minecraft');
    expect(lire(fixture, 'sans-licence')).toContain('thesam1798');
    expect(fixture.nativeElement.querySelector('[data-test="changer-de-compte"]')).not.toBeNull();
  });

  /** Dans cet état, les pas n'ont plus rien à dire : la suite est ailleurs. */
  it('sans licence, la barre des pas disparaît', () => {
    session.compte.set({ pseudo: 'x', uuid: '0', possedeLeJeu: false });
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="pas"]')).toBeNull();
  });
});
