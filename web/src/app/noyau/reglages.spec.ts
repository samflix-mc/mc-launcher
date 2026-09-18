import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Reglages } from './reglages';

describe('Reglages', () => {
  let reglages: Reglages;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    reglages = TestBed.inject(Reglages);
    document.documentElement.removeAttribute('data-fond');
    document.documentElement.style.removeProperty('--voile');
  });

  /**
   * Les défauts du front doublent ceux de Rust, et c'est une servitude
   * assumée : l'alternative — n'afficher aucun réglage tant que Rust n'a pas
   * répondu — ferait clignoter toute la page de configuration à chaque
   * ouverture.
   */
  it('part des mêmes défauts que Rust', () => {
    const vue = reglages.vue();

    expect(vue.schema).toBe(1);
    expect(vue.apparence.fond).toBe('spawn');
    expect(vue.apparence.voile).toBe(0.55);
    expect(vue.fenetre.mode).toBe('fenetree');
  });

  /**
   * **L'apparence est posée sur `<html>`, depuis ce service.**
   *
   * Pas par un style de composant : l'encapsulation émulée d'Angular réécrit
   * les sélecteurs, et un `:root { … }` écrit dans un composant deviendrait
   * `:root[_ngcontent-abc] { … }` — qui ne désigne rien. Le test porte donc
   * sur l'élément racine, et c'est le seul endroit où il peut porter.
   */
  it('pose le fond et le voile sur l’élément racine', async () => {
    await reglages.charger();

    const racine = document.documentElement;
    expect(racine.dataset['fond']).toBe('spawn');
    expect(racine.style.getPropertyValue('--voile')).toBe('0.55');
  });

  /**
   * Hors de Tauri, `enregistrer` garde ce qu'on lui donne et l'applique : il
   * n'y a personne pour le borner, et la page doit rester utilisable sous
   * `ng serve`.
   */
  it('applique ce qu’on enregistre', async () => {
    const voulu = {
      ...reglages.vue(),
      apparence: { fond: 'nether' as const, voile: 0.8 },
    };

    await reglages.enregistrer(voulu);

    expect(reglages.vue().apparence.fond).toBe('nether');
    expect(document.documentElement.dataset['fond']).toBe('nether');
    expect(document.documentElement.style.getPropertyValue('--voile')).toBe('0.8');
  });

  /**
   * `modifier` ne remplace QUE la section donnée : les autres survivent.
   * Sans cela, changer le fond remettrait la distance d'affichage à son
   * défaut, sans que rien ne le dise.
   */
  it('ne change que la section qu’on lui donne', async () => {
    await reglages.modifier({ apparence: { fond: 'fin', voile: 0.6 } });

    expect(reglages.vue().apparence.fond).toBe('fin');
    // Intactes.
    expect(reglages.vue().jeu.renderDistance).toBe(12);
    expect(reglages.vue().lanceur.memoireMo).toBe(4096);
  });
});
