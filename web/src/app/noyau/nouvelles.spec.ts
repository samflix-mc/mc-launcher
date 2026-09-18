import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Fil } from './contrats';
import { Nouvelles } from './nouvelles';

function fil(billets: Fil['billets']): Fil {
  return { billets, horsLigne: false, ecartes: [] };
}

describe('Nouvelles', () => {
  let nouvelles: Nouvelles;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    nouvelles = TestBed.inject(Nouvelles);
  });

  it('n’a rien avant d’avoir chargé', () => {
    expect(nouvelles.fil()).toBeNull();
    expect(nouvelles.derniere()).toBeNull();
    expect(nouvelles.chargement()).toBe(false);
  });

  /**
   * La carte de Spawn ne montre QUE le premier billet. Le tri — épinglés
   * d'abord, puis du plus récent au plus ancien — est fait en Rust, pour que
   * la mutation l'atteigne : ici, « le premier » est donc bien « celui qu'il
   * faut montrer », et ce service n'a rien à réordonner.
   */
  it('prend le premier billet pour la carte de Spawn', () => {
    nouvelles.fil.set(
      fil([
        {
          id: 'epingle',
          titre: 'Le plus important',
          date: '2026-09-01T00:00:00Z',
          epinglee: true,
          image: null,
          corps: [],
        },
        {
          id: 'recent',
          titre: 'Le plus récent',
          date: '2026-09-18T00:00:00Z',
          epinglee: false,
          image: null,
          corps: [],
        },
      ]),
    );

    expect(nouvelles.derniere()?.id).toBe('epingle');
  });

  /**
   * Un fil vide n'est pas une erreur — c'est un hôte qui ne publie pas encore
   * de nouvelles, et la page doit s'ouvrir dessus sans rien dire.
   */
  it('supporte un fil vide', () => {
    nouvelles.fil.set(fil([]));

    expect(nouvelles.derniere()).toBeNull();
    expect(nouvelles.fil()?.billets).toHaveLength(0);
  });

  /**
   * Hors de Tauri, `charger` ne demande rien et ne rejette pas. C'est aussi
   * ce qui garde la page utilisable sous `ng serve`.
   */
  it('hors de Tauri, ne demande rien', async () => {
    await expect(nouvelles.charger()).resolves.toBeUndefined();

    expect(nouvelles.fil()).toBeNull();
    expect(nouvelles.chargement()).toBe(false);
  });

  /**
   * Le fil est gardé pour la session : revenir sur la page ne le redemande
   * pas. Un fil change plusieurs fois par mois, pas plusieurs fois par
   * minute, et le recharger à chaque navigation ferait clignoter la page.
   */
  it('ne recharge pas un fil déjà là', async () => {
    const dejala = fil([]);
    nouvelles.fil.set(dejala);

    await nouvelles.charger();

    expect(nouvelles.fil()).toBe(dejala);
  });
});
