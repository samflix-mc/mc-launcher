import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Configuration } from './configuration';
import { Reglages } from '../noyau/reglages';

/**
 * La page de configuration.
 *
 * Deux choses s'y éprouvent, et ce sont les deux qui ont été signalées à la
 * recette : que les curseurs suivent le doigt, et que rien ne parte sur le
 * réseau avant le relâchement.
 */
describe('Configuration', () => {
  let reglages: Reglages;

  function monter() {
    const fixture = TestBed.createComponent(Configuration);
    fixture.detectChanges();
    return fixture;
  }

  function curseur(fixture: ReturnType<typeof monter>, test: string): HTMLInputElement {
    return fixture.nativeElement.querySelector(`[data-test="${test}"]`);
  }

  function lire(fixture: ReturnType<typeof monter>, test: string): string | null {
    const element = fixture.nativeElement.querySelector(`[data-test="${test}"]`);
    return element ? element.textContent.replace(/\s+/g, ' ').trim() : null;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
    reglages = TestBed.inject(Reglages);
  });

  /**
   * **Le défaut du curseur suit le brouillon, et le brouillon suit le
   * service.** Sans cette chaîne, la page s'ouvrirait sur les valeurs
   * d'origine et les écraserait au premier geste.
   */
  it('les contrôles affichent ce que le service porte', () => {
    const fixture = monter();

    expect(curseur(fixture, 'rendu').value).toBe(String(reglages.vue().jeu.renderDistance));
    expect(lire(fixture, 'voile-valeur')).toContain(
      `${(reglages.vue().apparence.voile * 100).toFixed(0)} %`,
    );
  });

  /**
   * **Le nombre affiché suit le doigt.**
   *
   * C'est la moitié visible du défaut signalé : « ça déplace le bouton, mais
   * ça ne change pas la valeur ». Le brouillon local est ce qui la corrige.
   */
  it('pendant le geste, la valeur affichée bouge', () => {
    const fixture = monter();
    const rendu = curseur(fixture, 'rendu');

    rendu.value = '24';
    rendu.dispatchEvent(new Event('input'));
    fixture.detectChanges();

    expect(lire(fixture, 'rendu-valeur')).toBe('24');
  });

  /**
   * **Et rien n'est écrit tant qu'on tire.**
   *
   * C'est l'autre moitié : la version précédente enregistrait sur `input`, soit
   * trente à cinquante écritures par seconde dont les réponses revenaient dans
   * le désordre — la plus ancienne écrasant la plus récente.
   */
  it('pendant le geste, rien n’est écrit', () => {
    const fixture = monter();
    const rendu = curseur(fixture, 'rendu');
    const avant = reglages.vue().jeu.renderDistance;

    for (const valeur of ['14', '18', '22', '26']) {
      rendu.value = valeur;
      rendu.dispatchEvent(new Event('input'));
    }
    fixture.detectChanges();

    expect(reglages.vue().jeu.renderDistance).toBe(avant);
  });

  /** Au relâchement, une seule écriture — et elle porte la dernière valeur. */
  it('au relâchement, la valeur est enregistrée', async () => {
    const fixture = monter();
    const rendu = curseur(fixture, 'rendu');

    rendu.value = '26';
    rendu.dispatchEvent(new Event('input'));
    rendu.dispatchEvent(new Event('change'));
    await fixture.whenStable();

    expect(reglages.vue().jeu.renderDistance).toBe(26);
  });

  /**
   * **Le voile descend jusqu'à zéro.**
   *
   * Il était borné à 0,44 du temps où c'était lui qui tenait le contraste. Ce
   * n'est plus le cas — le verre s'épaissit de lui-même — et un curseur qui
   * refuserait encore d'y descendre rendrait un geste qui saute.
   */
  it('le voile peut aller jusqu’à zéro', () => {
    const fixture = monter();

    expect(curseur(fixture, 'voile').min).toBe('0');
  });

  /**
   * La mémoire monte à soixante-quatre gigaoctets. Elle s'arrêtait à
   * trente-deux, ce qui coupait le curseur au milieu sur la machine de Sam sans
   * que rien ne dise pourquoi.
   */
  it('la mémoire monte à soixante-quatre gigaoctets', () => {
    const fixture = monter();

    expect(curseur(fixture, 'memoire').max).toBe('64');
  });

  /**
   * **Le rail ne met rien dans l'URL.**
   *
   * Ses entrées étaient des ancres ; avec le fragment, le routeur les prenait
   * pour des routes. Ce sont des boutons, et rien dans leur balisage ne doit
   * redevenir un lien.
   */
  it('le rail est fait de boutons, sans href', () => {
    const fixture = monter();

    const entrees = [...fixture.nativeElement.querySelectorAll('[data-test^="rail-"]')];
    expect(entrees.length).toBe(5);
    for (const entree of entrees as HTMLElement[]) {
      expect(entree.tagName).toBe('BUTTON');
      expect(entree.getAttribute('href')).toBeNull();
    }
  });

  /**
   * Une seule vérification, et c'est la complète. La « rapide » ne comparait
   * que les tailles : elle disait « tout est en place » sur un fichier corrompu
   * de la bonne longueur, c'est-à-dire dans le seul cas où l'on vérifie.
   */
  it('il n’y a qu’un seul bouton de vérification', () => {
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="verifier"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="verifier-rapide"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="verifier-profond"]')).toBeNull();
  });
});
