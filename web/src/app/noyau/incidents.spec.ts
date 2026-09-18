import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { Incidents } from './incidents';

describe('Incidents', () => {
  let incidents: Incidents;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    incidents = TestBed.inject(Incidents);
    // Le service journalise la cause brute dans la console. C'est voulu — on
    // veut les deux, le message mis en forme et la cause — mais une suite qui
    // crache des erreurs rouges finit par cacher les vraies.
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  it('n’a rien à montrer au départ', () => {
    expect(incidents.courant()).toBeNull();
    expect(incidents.ouvert()).toBe(false);
  });

  /**
   * Ce que Rust envoie est une CHAÎNE, et ce qui arrive autrement vient du
   * pont. Sans cette traduction, un joueur lirait « [object Object] ».
   */
  it('traduit ce qu’on lui donne', () => {
    incidents.signaler('le verrou est injoignable');
    expect(incidents.courant()).toBe('le verrou est injoignable');

    incidents.signaler(new Error('java 21 introuvable'));
    expect(incidents.courant()).toBe('java 21 introuvable');
  });

  /**
   * Une seule erreur à la fois, et c'est la DERNIÈRE qui gagne.
   *
   * Empiler demanderait au joueur de les fermer une par une, alors que la
   * première est presque toujours la cause des suivantes.
   */
  it('garde la dernière, pas la première', () => {
    incidents.signaler('première');
    incidents.signaler('seconde');

    expect(incidents.courant()).toBe('seconde');
    expect(incidents.ouvert()).toBe(true);
  });

  it('se ferme', () => {
    incidents.signaler('quelque chose');
    incidents.fermer();

    expect(incidents.courant()).toBeNull();
    expect(incidents.ouvert()).toBe(false);
  });

  it('laisse passer ce qui réussit, sans rien signaler', async () => {
    const valeur = await incidents.pendant(async () => 42);

    expect(valeur).toBe(42);
    expect(incidents.ouvert()).toBe(false);
  });

  /**
   * **Le cas qui justifie l'enveloppe.** Chaque appel refaisait la même
   * séquence, avec chaque fois une occasion d'oublier le `catch` — et une
   * promesse rejetée dans le vide, avec une erreur que personne ne voit.
   */
  it('attrape ce qui échoue, le montre, et rend null', async () => {
    const valeur = await incidents.pendant(async () => {
      throw new Error('le pack a disparu');
    });

    expect(valeur).toBeNull();
    expect(incidents.courant()).toBe('le pack a disparu');
  });

  /**
   * L'enveloppe efface l'erreur précédente AVANT d'agir : sans cela, un
   * second essai réussi laisserait l'overlay du premier échec affiché.
   */
  it('efface l’erreur précédente avant d’agir', async () => {
    incidents.signaler('échec d’avant');

    await incidents.pendant(async () => 'ça marche');

    expect(incidents.ouvert()).toBe(false);
  });
});
