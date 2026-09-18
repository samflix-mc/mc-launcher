import { describe, expect, it } from 'vitest';

import { messageDErreur, teteDuJoueur } from './pont';

/**
 * Deux fonctions libres, éprouvées SANS monter de composant.
 *
 * L'ancienne suite montait le composant entier pour vérifier ces deux
 * fonctions. Les tester ici est une redistribution par sujet, pas un
 * déménagement : ce qu'elles font n'a rien à voir avec un DOM.
 */

describe('messageDErreur', () => {
  /**
   * Le cas courant : `Erreur` se sérialise en chaîne côté Rust, et c'est
   * exactement ce qu'on veut afficher.
   */
  it('rend une chaîne telle quelle', () => {
    expect(messageDErreur('la session a expiré')).toBe('la session a expiré');
  });

  it('prend le message d’une Error', () => {
    expect(messageDErreur(new Error('réseau injoignable'))).toBe('réseau injoignable');
  });

  /**
   * `String({})` donnerait « [object Object] », qui n'apprend rien à personne
   * — et c'est précisément ce qu'un joueur recopierait dans un rapport.
   */
  it('n’affiche jamais « [object Object] »', () => {
    expect(messageDErreur({ code: 42 })).not.toContain('[object Object]');
    expect(messageDErreur({ code: 42 })).toContain('42');
  });

  it('survit à ce qui ne se sérialise pas', () => {
    const cyclique: Record<string, unknown> = {};
    cyclique['soi'] = cyclique;
    expect(() => messageDErreur(cyclique)).not.toThrow();
  });
});

describe('teteDuJoueur', () => {
  it('construit l’adresse de mc-heads', () => {
    expect(teteDuJoueur('abc-123')).toBe('https://mc-heads.net/head/abc-123/96');
  });

  /**
   * L'UUID vient de Microsoft, donc de l'extérieur. Sans encodage, un
   * identifiant contenant une barre oblique ou un point d'interrogation
   * sortirait du chemin attendu.
   */
  it('encode ce qui vient de l’extérieur', () => {
    expect(teteDuJoueur('a/b?c')).toBe('https://mc-heads.net/head/a%2Fb%3Fc/96');
  });

  it('accepte une autre taille', () => {
    expect(teteDuJoueur('abc', 32)).toContain('/32');
  });
});
