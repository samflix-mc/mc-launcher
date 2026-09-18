import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Marque } from '../../noyau/marque';
import { Notifications } from '../../noyau/notifications';
import { Pack } from '../../noyau/pack';
import { BarreTitre } from './barre-titre';

/**
 * La barre de titre, dans ses deux formes.
 *
 * Celle de la fenêtre principale porte le nom du modpack, la cloche et les
 * trois contrôles ; celle d'une fenêtre de dialogue perd la cloche et le bouton
 * d'agrandissement.
 */
describe('BarreTitre', () => {
  function monter(dialogue = false) {
    const fixture = TestBed.createComponent(BarreTitre);
    fixture.componentRef.setInput('dialogue', dialogue);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
  });

  it('elle porte le nom du launcher', () => {
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="titre"]').textContent).toContain(
      TestBed.inject(Marque).vue().nom,
    );
  });

  /**
   * Ce qui suit le séparateur est le nom du MODPACK — ce que le joueur
   * reconnaît, et ce qui change quand le serveur change de saison. Le design
   * system y met le nom du serveur ; nous n'en avons qu'un.
   */
  it('le nom du pack suit le séparateur, quand il est connu', () => {
    expect(monter().nativeElement.querySelector('[data-test="pack"]')).toBeNull();

    TestBed.inject(Pack).etat.set({
      action: 'jouer',
      ecart: 'a-jour',
      horsLigne: false,
      installe: true,
      nom: 'samflix',
      version: '1.4.2',
      java: 21,
      mods: 128,
      generation: 1,
    });
    expect(monter().nativeElement.querySelector('[data-test="pack"]').textContent).toContain(
      'samflix',
    );
  });

  /** La pastille est cachée à zéro : « 0 » demande à être lu pour rien. */
  it('la cloche ne compte que ce qui n’est pas lu', () => {
    expect(monter().nativeElement.querySelector('[data-test="non-lus"]')).toBeNull();

    TestBed.inject(Notifications).signaler('info', 'Un');
    expect(monter().nativeElement.querySelector('[data-test="non-lus"]').textContent).toContain(
      '1',
    );
  });

  it('les trois contrôles de fenêtre sont là', () => {
    const fixture = monter();

    for (const nom of ['reduire', 'maximiser', 'fermer']) {
      expect(fixture.nativeElement.querySelector(`[data-test="${nom}"]`)).not.toBeNull();
    }
  });

  /**
   * **Dans une fenêtre de dialogue, la cloche part et le titre change.**
   *
   * Elle n'a rien à annoncer là où l'on ne fait qu'une chose, et ce qui suit le
   * séparateur est ce qu'on y FAIT — le nom du modpack n'y dirait rien.
   */
  it('en dialogue, pas de cloche, et le titre dit ce qu’on y fait', () => {
    const fixture = monter(true);

    expect(fixture.nativeElement.querySelector('[data-test="cloche"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="pack"]').textContent).toContain(
      'Connexion',
    );
  });

  /**
   * `--dialog` cache le bouton d'agrandissement, et `--glass` donne à la barre
   * de quoi se voir sur une feuille dépolie sans image derrière elle.
   */
  it('en dialogue, la barre prend ses deux modificateurs', () => {
    const barre = monter(true).nativeElement.querySelector('[data-test="barre-titre"]');

    expect(barre.className).toContain('hm-titlebar--dialog');
    expect(barre.className).toContain('hm-titlebar--glass');
  });
});
