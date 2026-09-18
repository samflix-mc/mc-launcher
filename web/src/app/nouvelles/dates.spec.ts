import { describe, expect, it } from 'vitest';

import { direLaDate } from './dates';

/**
 * L'instant de référence est PASSÉ EN PARAMÈTRE, et c'est ce qui rend ces
 * tests possibles. Sans lui, il faudrait truquer l'horloge, et les résultats
 * dépendraient du jour où l'on lance la suite.
 */
const MAINTENANT = new Date('2026-09-18T12:00:00Z');

describe('direLaDate', () => {
  it('dit « à l’instant » pour ce qui vient de se produire', () => {
    expect(direLaDate('2026-09-18T11:59:30Z', MAINTENANT)).toBe("à l'instant");
  });

  it('compte en minutes, puis en heures, puis en jours', () => {
    expect(direLaDate('2026-09-18T11:30:00Z', MAINTENANT)).toContain('30');
    expect(direLaDate('2026-09-18T09:00:00Z', MAINTENANT)).toContain('3');
  });

  /**
   * `numeric: 'auto'` rend « hier » et « avant-hier » plutôt que « il y a 1 /
   * 2 jour(s) », et c'est ce qu'on veut : une langue dit les jours proches par
   * leur nom, pas par un compte.
   */
  it('dit « hier » et « avant-hier » plutôt qu’un compte', () => {
    expect(direLaDate('2026-09-17T12:00:00Z', MAINTENANT)).toBe('hier');
    expect(direLaDate('2026-09-16T12:00:00Z', MAINTENANT)).toBe('avant-hier');
  });

  /** Au-delà, un compte de jours — jusqu'à la bascule d'une semaine. */
  it('compte les jours au-delà d’avant-hier', () => {
    expect(direLaDate('2026-09-14T12:00:00Z', MAINTENANT)).toContain('4');
  });

  /**
   * Au-delà d'une semaine, une date et non un compte.
   *
   * « il y a 47 jours » demande un calcul mental que « 2 août » ne demande
   * pas : c'est à peu près l'horizon où l'on cesse de compter.
   */
  it('bascule sur une date au-delà d’une semaine', () => {
    const dit = direLaDate('2026-08-02T12:00:00Z', MAINTENANT);
    expect(dit).toContain('août');
    // Même année : l'année n'est pas répétée, elle n'apprendrait rien.
    expect(dit).not.toContain('2026');
  });

  it('ajoute l’année quand ce n’est pas celle en cours', () => {
    expect(direLaDate('2024-08-02T12:00:00Z', MAINTENANT)).toContain('2024');
  });

  /**
   * Rust valide déjà le format, mais un fil servi par un hôte modifié pourrait
   * passer autre chose. Afficher « Invalid Date » à un joueur serait pire que
   * de ne rien afficher du tout.
   */
  it('ne dit rien plutôt que « Invalid Date »', () => {
    expect(direLaDate('pas une date', MAINTENANT)).toBe('');
    expect(direLaDate('', MAINTENANT)).toBe('');
  });
});
