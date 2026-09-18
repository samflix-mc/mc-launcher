import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import type { EtatDuPack, Partie } from '../noyau/contrats';
import { Nouvelles } from '../noyau/nouvelles';
import { Pack } from '../noyau/pack';
import { Spawn } from './spawn';

function etat(dessus: Partial<EtatDuPack> = {}): EtatDuPack {
  return {
    action: 'jouer',
    ecart: 'a-jour',
    horsLigne: false,
    installe: true,
    nom: 'samflix',
    version: '1.4.2',
    java: 21,
    mods: 128,
    generation: 1,
    ...dessus,
  };
}

function partie(dessus: Partial<Partie> = {}): Partie {
  return {
    verdict: 'La partie s’est terminée normalement.',
    rattrapee: false,
    introuvables: [],
    ecarts: [],
    horsLigne: false,
    purge: [],
    ...dessus,
  };
}

/**
 * La page Spawn.
 *
 * Ce qu'elle doit montrer tient en une phrase : l'état du pack, lisiblement, et
 * ce que la dernière installation a laissé — sans rien de ce que la barre du bas
 * porte déjà.
 */
describe('Spawn', () => {
  let pack: Pack;

  function monter() {
    const fixture = TestBed.createComponent(Spawn);
    fixture.detectChanges();
    return fixture;
  }

  function lire(fixture: ReturnType<typeof monter>, test: string): string | null {
    const element = fixture.nativeElement.querySelector(`[data-test="${test}"]`);
    return element ? element.textContent.replace(/\s+/g, ' ').trim() : null;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
    pack = TestBed.inject(Pack);
  });

  /**
   * **La pastille dit l'état en trois mots, et jamais par la couleur seule.**
   *
   * Le design system l'interdit explicitement : `success` et `danger` ne se
   * distinguent que par la teinte, et un libellé est ce qui fait la différence
   * pour qui la perçoit mal.
   */
  it('à jour : la pastille le dit, avec la version', () => {
    pack.etat.set(etat());
    const fixture = monter();

    expect(lire(fixture, 'pastille')).toContain('À jour');
    expect(lire(fixture, 'pastille')).toContain('1.4.2');
  });

  it('pas installé : la pastille avertit', () => {
    pack.etat.set(etat({ ecart: 'absent', installe: false }));
    const fixture = monter();

    const pastille = fixture.nativeElement.querySelector('[data-test="pastille"]');
    expect(pastille.textContent).toContain('Pas installé');
    expect(pastille.dataset.etat).toBe('warning');
  });

  /**
   * Hors ligne l'emporte sur l'écart : ce qu'on croit savoir du pack publié
   * n'a pas été vérifié, et l'annoncer « à jour » serait une affirmation qu'on
   * n'a pas les moyens de faire.
   */
  it('hors ligne l’emporte sur l’écart', () => {
    pack.etat.set(etat({ horsLigne: true, ecart: 'a-jour' }));
    const fixture = monter();

    const pastille = fixture.nativeElement.querySelector('[data-test="pastille"]');
    expect(pastille.textContent).toContain('Hors ligne');
    expect(pastille.dataset.etat).toBe('offline');
  });

  it('avant la première réponse, la pastille dit qu’on regarde', () => {
    const fixture = monter();

    const pastille = fixture.nativeElement.querySelector('[data-test="pastille"]');
    expect(pastille.dataset.etat).toBe('unknown');
    expect(pastille.textContent).toContain('Vérification');
  });

  it('les faits du pack s’affichent quand ils sont connus', () => {
    pack.etat.set(etat());
    const fixture = monter();

    expect(lire(fixture, 'mods')).toContain('128 mods');
    expect(lire(fixture, 'java')).toContain('Java 21');
    expect(lire(fixture, 'installe')).toContain('Installé');
  });

  /**
   * **La cinématique n'est plus ici**, et ce test l'y garde.
   *
   * Le bouton porte la progression, son pourcentage et son débit, et la phrase
   * au-dessus de lui nomme l'étape : une liste des onze phases le redirait une
   * troisième fois, au prix d'un panneau qui apparaît et disparaît à l'endroit
   * même où l'on suit l'avancement.
   */
  it('aucune liste d’étapes, même pendant le travail', () => {
    pack.etat.set(etat());
    pack.occupe.set(true);
    pack.chemin.set([{ phase: 'mods', libelle: 'Mods', rang: 0 }]);
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="chemin"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="panneau-etapes"]')).toBeNull();
  });

  /**
   * Le compte rendu ne s'affiche QUE s'il a quelque chose à dire. Une partie
   * sans incident laisse un verdict que personne n'a besoin de lire, et un
   * panneau vide occuperait la place que l'image doit garder.
   */
  it('une partie sans incident ne laisse aucun panneau', () => {
    pack.etat.set(etat());
    pack.derniereePartie.set(partie());
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="panneau-partie"]')).toBeNull();
  });

  it('des mods introuvables se disent, avec leurs noms', () => {
    pack.etat.set(etat());
    pack.derniereePartie.set(partie({ introuvables: ['sodium', 'iris'] }));
    const fixture = monter();

    expect(lire(fixture, 'introuvables')).toContain('2 mod(s) introuvable(s)');
    expect(lire(fixture, 'introuvables')).toContain('sodium');
  });

  it('une purge se dit, et rassure sur ce qui a été gardé', () => {
    pack.etat.set(etat());
    pack.derniereePartie.set(partie({ purge: ['mods', 'config'] }));
    const fixture = monter();

    expect(lire(fixture, 'purge')).toContain('mods, config');
    expect(lire(fixture, 'purge')).toContain('mondes');
  });

  /**
   * Sans fil de nouvelles, la grille garderait un trou. Un panneau qui le dit
   * tient la place et ne prétend rien.
   */
  it('sans nouvelle, la place est tenue par un panneau qui le dit', () => {
    TestBed.inject(Nouvelles);
    const fixture = monter();

    expect(lire(fixture, 'sans-nouvelle')).toContain('Rien de publié');
  });
});
