import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Fenetre } from './fenetre';
import { Pont } from './pont';

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

  /**
   * **La régression qui a motivé ce test.**
   *
   * `disponible` voulait dire « dans Tauri » jusqu'à ce que le serveur de
   * développement existe ; il veut maintenant dire « il y a un backend ». Dans
   * le navigateur, il est donc VRAI — et la garde qui protégeait
   * `getCurrentWindow()` a cessé de protéger. Le symptôme : un `TypeError:
   * Cannot read properties of undefined (reading 'metadata')` au démarrage,
   * remonté dans l'overlay d'incidents avant même le premier écran.
   *
   * Ce service pilote la FENÊTRE : il doit donc lire `dansLaFenetre`, et non
   * `disponible`. Le test le vérifie en posant exactement la situation du
   * navigateur — un backend joignable, pas de fenêtre.
   */
  it('ne touche pas à la fenêtre quand il y a un backend mais pas de fenêtre', async () => {
    const pont = TestBed.inject(Pont);
    // Le cas du navigateur branché sur le serveur de développement.
    Object.defineProperty(pont, 'disponible', { value: true, configurable: true });
    Object.defineProperty(pont, 'dansLaFenetre', { value: false, configurable: true });

    await expect(fenetre.observer()).resolves.toBeUndefined();
    await expect(fenetre.reduire()).resolves.toBeUndefined();
    await expect(fenetre.basculerMaximisee()).resolves.toBeUndefined();
    await expect(fenetre.fermer()).resolves.toBeUndefined();
  });
});
