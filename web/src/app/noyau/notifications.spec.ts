import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { Notifications } from './notifications';

describe('Notifications', () => {
  let service: Notifications;

  beforeEach(() => {
    vi.useFakeTimers();
    TestBed.configureTestingModule({});
    service = TestBed.inject(Notifications);
  });

  /**
   * **La règle du design system : tout toast est AUSSI écrit au centre.**
   *
   * Un toast dure six secondes, et six secondes suffisent à regarder ailleurs.
   * Si `signaler` n'écrivait qu'à l'écran, un téléchargement échoué pendant
   * qu'on lit les nouvelles disparaîtrait sans laisser de trace.
   */
  it('un toast entre aussi au centre', () => {
    service.signaler('success', 'Pack mis à jour', '128 mods');

    expect(service.toasts().length).toBe(1);
    expect(service.journal().length).toBe(1);
    expect(service.journal()[0].titre).toBe('Pack mis à jour');
  });

  /**
   * L'inverse n'est pas vrai, et c'est ce qui permet aux incidents de ne pas
   * se dire deux fois : le dialogue les montre déjà en grand.
   */
  it('archiver écrit au centre sans ouvrir de toast', () => {
    service.archiver('danger', "Quelque chose n'a pas fonctionné", 'détail');

    expect(service.toasts().length).toBe(0);
    expect(service.journal().length).toBe(1);
  });

  /**
   * Les durées viennent du design system : six secondes pour ce qui informe,
   * douze pour ce qui demande une décision. Un toast d'erreur qui s'effacerait
   * en six secondes emporterait avec lui la seule chose à lire.
   */
  it('un toast de succès part après six secondes, un toast de danger tient douze', () => {
    service.signaler('success', 'Fait');
    service.signaler('danger', 'Échec');

    vi.advanceTimersByTime(6000);
    expect(service.toasts().map((avis) => avis.titre)).toEqual(['Échec']);

    vi.advanceTimersByTime(6000);
    expect(service.toasts()).toEqual([]);
    // Partis de l'écran, toujours au centre.
    expect(service.journal().length).toBe(2);
  });

  /**
   * Un toast de progression reste jusqu'à ce que le travail finisse : c'est
   * l'appelant qui le retire. Le voir disparaître au bout de six secondes
   * pendant un téléchargement de huit cents mégaoctets serait le contraire de
   * ce qu'il annonce.
   */
  it('un toast de progression ne part pas tout seul', () => {
    const id = service.signaler('progress', 'Téléchargement');

    vi.advanceTimersByTime(60_000);
    expect(service.toasts().length).toBe(1);

    service.fermerToast(id);
    expect(service.toasts()).toEqual([]);
  });

  /**
   * **Le plus récent EN BAS.**
   *
   * `.hm-toasts` aligne son contenu en bas et la pile monte : l'œil revient
   * toujours au même endroit. L'ordre du tableau porte donc cette règle, et
   * l'inverser ferait apparaître les nouveaux toasts en haut de la pile,
   * c'est-à-dire là où l'on ne regarde pas.
   */
  it('les toasts sont rendus du plus ancien au plus récent', () => {
    service.signaler('info', 'Premier');
    service.signaler('info', 'Deuxième');
    service.signaler('info', 'Troisième');

    expect(service.toasts().map((avis) => avis.titre)).toEqual([
      'Premier',
      'Deuxième',
      'Troisième',
    ]);
  });

  /** Trois à l'écran au plus ; les autres attendent au centre. */
  it('au plus trois toasts se voient à la fois', () => {
    for (let rang = 1; rang <= 5; rang += 1) {
      service.signaler('info', `Avis ${rang}`);
    }

    expect(service.toasts().length).toBe(3);
    expect(service.journal().length).toBe(5);
  });

  /**
   * Ouvrir le panneau, c'est avoir vu ce qu'il porte. Marquer à la FERMETURE
   * laisserait la pastille allumée pendant qu'on lit dessous.
   */
  it('ouvrir le panneau marque tout comme lu', () => {
    service.signaler('info', 'Un');
    service.signaler('info', 'Deux');
    expect(service.nonLus()).toBe(2);

    service.basculerPanneau();

    expect(service.panneauOuvert()).toBe(true);
    expect(service.nonLus()).toBe(0);
  });

  /**
   * Cinquante, comme le design system le pose. Sans borne, un launcher laissé
   * ouvert une semaine accumulerait un journal que personne ne déroule.
   */
  it('le centre ne garde que les cinquante derniers', () => {
    for (let rang = 1; rang <= 60; rang += 1) {
      service.archiver('info', `Avis ${rang}`);
    }

    expect(service.journal().length).toBe(50);
    // Le plus récent en tête : c'est le soixantième qui reste, pas le premier.
    expect(service.journal()[0].titre).toBe('Avis 60');
  });

  it('vider efface le centre et les toasts', () => {
    service.signaler('info', 'Un');
    service.vider();

    expect(service.journal()).toEqual([]);
    expect(service.toasts()).toEqual([]);
  });
});
