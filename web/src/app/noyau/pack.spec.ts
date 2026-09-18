import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Avancement, EtatDuPack } from './contrats';
import { Pack } from './pack';

function etat(partiel: Partial<EtatDuPack>): EtatDuPack {
  return {
    action: 'jouer',
    ecart: 'a-jour',
    horsLigne: false,
    installe: true,
    nom: 'samflix',
    version: '3.2',
    java: 21,
    mods: 120,
    generation: 0,
    ...partiel,
  };
}

function avancement(partiel: Partial<Avancement>): Avancement {
  return {
    phase: 'mods',
    achevee: false,
    note: null,
    fichier: null,
    octets: 0,
    total: 0,
    fichiers: 0,
    fichiersTotal: 0,
    actif: false,
    debit: 0,
    restant: null,
    ...partiel,
  };
}

describe('Pack — le bouton', () => {
  let pack: Pack;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    pack = TestBed.inject(Pack);
  });

  /**
   * LE test du bouton, et il porte sur le cas qu'on oublie.
   *
   * Entre l'affichage de Spawn et la réponse d'`etat_du_pack()`, le bouton
   * doit dire qu'il REGARDE. Une valeur par défaut « Installer » produirait un
   * clignotement INSTALLER → JOUER à chaque démarrage, sur le seul élément de
   * l'écran qui compte.
   */
  it('dit « inconnu » tant que l’état n’est pas connu', () => {
    expect(pack.bouton()).toBe('inconnu');
  });

  it('suit ce que le disque impose', () => {
    pack.etat.set(etat({ action: 'installer' }));
    expect(pack.bouton()).toBe('installer');

    pack.etat.set(etat({ action: 'jouer' }));
    expect(pack.bouton()).toBe('jouer');
  });

  /**
   * L'ACTIVITÉ l'emporte sur l'action.
   *
   * L'action dit ce que le disque impose ; elle ne dit rien d'une installation
   * en cours. Les confondre ferait proposer JOUER pendant qu'on télécharge.
   */
  it('l’occupation l’emporte sur l’action', () => {
    pack.etat.set(etat({ action: 'jouer' }));
    pack.occupe.set(true);
    expect(pack.bouton()).toBe('occupe');
  });

  /**
   * Et la PARTIE l'emporte sur tout — c'est le plus long état de la session,
   * et celui qu'aucun champ du modèle ne portait avant.
   */
  it('la partie l’emporte sur l’occupation', () => {
    pack.etat.set(etat({ action: 'jouer' }));
    pack.occupe.set(true);
    pack.avancement.set(avancement({ phase: 'lancement' }));
    expect(pack.bouton()).toBe('en-partie');
  });

  /**
   * `enPartie` est DÉRIVÉ et non posé : un signal qu'on doit penser à remettre
   * à zéro se retrouve un jour bloqué à `true`, et le bouton reste « En jeu »
   * pour le reste de la session.
   */
  it('quitte « en jeu » quand la cinématique revient à « prêt »', () => {
    pack.avancement.set(avancement({ phase: 'lancement' }));
    expect(pack.enPartie()).toBe(true);

    pack.avancement.set(avancement({ phase: 'pret', achevee: true }));
    expect(pack.enPartie()).toBe(false);
  });
});

describe('Pack — la cinématique', () => {
  let pack: Pack;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    pack = TestBed.inject(Pack);
    pack.chemin.set([
      { phase: 'pack', libelle: 'Pack', rang: 0 },
      { phase: 'mods', libelle: 'Mods', rang: 1 },
      { phase: 'pret', libelle: 'Prêt', rang: 2 },
    ]);
  });

  it('marque ce qui est derrière, en cours, et à venir', () => {
    pack.avancement.set(avancement({ phase: 'mods', achevee: false }));

    const etats = pack.etapes().map((e) => e.etat);
    expect(etats).toEqual(['faite', 'en-cours', 'a-venir']);
  });

  /**
   * Une phase ACHEVÉE est derrière nous, pas en cours : « prêt à jouer » ne
   * doit pas clignoter comme s'il restait à l'attendre.
   */
  it('une phase achevée est derrière, pas en cours', () => {
    pack.avancement.set(avancement({ phase: 'mods', achevee: true }));

    const etats = pack.etapes().map((e) => e.etat);
    expect(etats).toEqual(['faite', 'faite', 'a-venir']);
  });

  /**
   * « Prêt » est un ABOUTISSEMENT : il ne compte pas dans la progression,
   * parce qu'il n'est pas une étape qu'on exécute mais le résultat des autres.
   */
  it('les aboutissements ne comptent pas dans la progression', () => {
    pack.avancement.set(avancement({ phase: 'pack', total: 100, octets: 100, actif: true }));
    // Deux étapes utiles sur trois : finir la première fait 50 %.
    expect(Math.round(pack.progression())).toBe(50);
  });
});
