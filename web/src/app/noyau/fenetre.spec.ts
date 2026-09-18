import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Fenetre } from './fenetre';

describe('Fenetre', () => {
  let fenetre: Fenetre;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    fenetre = TestBed.inject(Fenetre);
  });

  it('se croit fenêtrée tant qu’on ne lui a rien dit', () => {
    expect(fenetre.maximisee()).toBe(false);
  });

  /**
   * **Hors de Tauri, les quatre gestes ne font rien — et surtout ne jettent
   * pas.**
   *
   * Ce n'est pas un détail de confort : la barre de titre est montée par la
   * coque, donc elle existe aussi sous `ng serve`, où `getCurrentWindow()`
   * n'a pas d'interlocuteur. Sans la garde `disponible`, un clic sur le bouton
   * fermer y produirait une promesse rejetée qui remonterait dans l'overlay
   * d'incidents — un message d'erreur pour un bouton qui n'a rien à faire.
   */
  it('hors de Tauri, les gestes de fenêtre ne font rien', async () => {
    await expect(fenetre.reduire()).resolves.toBeUndefined();
    await expect(fenetre.basculerMaximisee()).resolves.toBeUndefined();
    await expect(fenetre.fermer()).resolves.toBeUndefined();
    await expect(fenetre.observer()).resolves.toBeUndefined();

    // Et l'état n'a pas bougé : rien n'a été maximisé.
    expect(fenetre.maximisee()).toBe(false);
  });
});
