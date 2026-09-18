import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { Nav } from './nav';

/**
 * La pilule de navigation.
 *
 * Trois entrées, dans cet ordre, toujours visibles — c'est une règle du design
 * system, et une quatrième section devrait d'abord se demander si elle
 * n'appartient pas à la Configuration.
 */
describe('Nav', () => {
  function monter() {
    const fixture = TestBed.createComponent(Nav);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
  });

  it('porte les trois sections, dans l’ordre', () => {
    const fixture = monter();

    const entrees = [...fixture.nativeElement.querySelectorAll('[data-test^="onglet-"]')];
    expect(entrees.map((e: Element) => e.textContent?.trim())).toEqual([
      'Spawn',
      'Nouvelles',
      'Configuration',
    ]);
  });

  /**
   * Des ANCRES et non des boutons : elles ont une URL, et une URL se copie,
   * s'ouvre au milieu, se garde dans l'historique. Le design system en dessine
   * des boutons ; la seule chose à reprendre est de retirer le soulignement.
   */
  it('les entrées sont des liens vers de vraies routes', () => {
    const fixture = monter();

    const entrees = [...fixture.nativeElement.querySelectorAll('[data-test^="onglet-"]')];
    for (const entree of entrees as HTMLElement[]) {
      expect(entree.tagName).toBe('A');
      expect(entree.getAttribute('href')).toMatch(/^\/(spawn|nouvelles|configuration)$/);
    }
  });

  it('chaque entrée est atteignable par son attribut de test', () => {
    const fixture = monter();

    for (const nom of ['spawn', 'nouvelles', 'configuration']) {
      expect(fixture.nativeElement.querySelector(`[data-test="onglet-${nom}"]`)).not.toBeNull();
    }
  });
});
