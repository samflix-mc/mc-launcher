import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Compte } from './contrats';
import { Session } from './session';

function compte(possedeLeJeu: boolean): Compte {
  return {
    pseudo: 'thesam1798',
    uuid: '0123-4567',
    possedeLeJeu,
  };
}

describe('Session', () => {
  let session: Session;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    session = TestBed.inject(Session);
  });

  /**
   * `undefined` n'est pas `null`, et la différence porte les deux gardes :
   * « on n'a pas encore demandé » n'est pas « personne n'est connecté ».
   * Les confondre ferait rediriger vers la connexion avant même d'avoir
   * interrogé Rust, à chaque démarrage.
   */
  it('ne prétend rien savoir avant d’avoir demandé', () => {
    expect(session.connue()).toBe(false);
    expect(session.jouable()).toBe(false);
    expect(session.sansLicence()).toBe(false);
  });

  it('sait quand personne n’est connecté', () => {
    session.compte.set(null);

    expect(session.connue()).toBe(true);
    expect(session.jouable()).toBe(false);
    expect(session.sansLicence()).toBe(false);
  });

  /**
   * **LE prédicat des deux gardes.** Un seul, et c'est délibéré : deux
   * prédicats différents feraient rebondir sans fin un compte connecté sans
   * licence entre `/connexion` et `/spawn`.
   */
  it('n’est jouable qu’avec la licence', () => {
    session.compte.set(compte(true));
    expect(session.jouable()).toBe(true);
    expect(session.sansLicence()).toBe(false);
  });

  /**
   * Le cas qui n'est pas théorique : un compte Microsoft valide qui n'a jamais
   * acheté Minecraft. C'est un ÉTAT à afficher, pas une redirection.
   */
  it('distingue « sans licence » de « pas connecté »', () => {
    session.compte.set(compte(false));

    expect(session.connue()).toBe(true);
    expect(session.jouable()).toBe(false);
    expect(session.sansLicence()).toBe(true);
  });

  /**
   * Hors de la fenêtre Tauri — `ng serve`, une suite — il n'y a personne à
   * qui demander. Rendre `null` plutôt que de rester à `undefined` évite que
   * l'écran attende une réponse qui ne viendra jamais.
   */
  it('hors de Tauri, conclut qu’il n’y a pas de compte', async () => {
    await session.ouvrir();

    expect(session.connue()).toBe(true);
    expect(session.compte()).toBeNull();
  });
});
