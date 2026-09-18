import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Marque } from './marque';

describe('Marque', () => {
  let marque: Marque;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    marque = TestBed.inject(Marque);
  });

  /**
   * Un défaut qui n'est PAS « chargement… ».
   *
   * La barre de titre est visible dès la première image : y afficher un texte
   * d'attente ferait clignoter le nom du launcher à chaque ouverture. Le
   * défaut est donc le nom probable, remplacé sans qu'on le remarque.
   */
  it('affiche un nom plausible avant d’avoir demandé', () => {
    expect(marque.vue().nom).toBe('Helm');
    expect(marque.vue().sceau).toBe('HE');
  });

  /**
   * Le sceau tient en deux caractères : c'est une pastille, pas un libellé.
   * Un sceau plus long déborderait du carré de la barre de titre.
   */
  it('a un sceau court', () => {
    expect(marque.vue().sceau.length).toBeLessThanOrEqual(2);
    expect(marque.vue().nom.length).toBeGreaterThan(0);
  });

  /**
   * Hors de la fenêtre Tauri, il n'y a personne à qui demander : le défaut
   * reste, et `charger` ne rejette pas. C'est le cas de `ng serve` et celui
   * de cette suite.
   */
  it('hors de Tauri, garde son défaut sans se plaindre', async () => {
    await expect(marque.charger()).resolves.toBeUndefined();
    expect(marque.vue().nom).toBe('Helm');
  });
});
