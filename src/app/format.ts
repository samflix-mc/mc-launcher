/**
 * Mettre des nombres sous une forme qu'on lit d'un coup d'œil.
 *
 * Rien ici n'est propre au launcher, et rien n'appelle Rust : ce sont des
 * fonctions pures, précisément pour qu'elles se vérifient sans fenêtre.
 */

const UNITES = ['o', 'ko', 'Mo', 'Go', 'To'] as const;

/**
 * Une taille en octets, dans l'unité qui la rend lisible.
 *
 * Multiples de 1000 et non de 1024 : c'est ce qu'annoncent les sites d'où
 * viennent les fichiers, et un joueur qui compare notre « 830 Mo » à leur
 * « 830 MB » doit trouver le même nombre.
 *
 * Une décimale à partir du mégaoctet, aucune en dessous : « 1,5 Mo » se lit,
 * « 1536,0 ko » non.
 */
export function octets(valeur: number): string {
  if (!Number.isFinite(valeur) || valeur <= 0) {
    return '0 o';
  }

  let reste = valeur;
  let rang = 0;
  while (reste >= 1000 && rang < UNITES.length - 1) {
    reste /= 1000;
    rang += 1;
  }

  const decimales = rang >= 2 && reste < 100 ? 1 : 0;
  return `${reste.toFixed(decimales).replace('.', ',')} ${UNITES[rang]}`;
}

/** Un débit, à la seconde. */
export function debit(octetsParSeconde: number): string {
  return `${octets(octetsParSeconde)}/s`;
}

/**
 * Une durée, en français et sans unité inutile.
 *
 * Au-delà d'une minute, les secondes sont données aussi : « 3 min » laisserait
 * croire à une précision qu'on n'a pas, et « 3 min 12 s » se lit comme un
 * compte à rebours. Au-delà de l'heure, on arrondit — à ce stade, la minute
 * près n'apporte rien.
 */
export function duree(secondes: number): string {
  if (!Number.isFinite(secondes) || secondes < 0) {
    return '—';
  }

  const entier = Math.round(secondes);
  if (entier < 60) {
    return `${entier} s`;
  }
  if (entier < 3600) {
    const minutes = Math.floor(entier / 60);
    const reste = entier % 60;
    return reste === 0 ? `${minutes} min` : `${minutes} min ${reste} s`;
  }

  const heures = Math.floor(entier / 3600);
  const minutes = Math.round((entier % 3600) / 60);
  return minutes === 0 ? `${heures} h` : `${heures} h ${minutes} min`;
}

/**
 * Une fraction en pourcentage, bornée.
 *
 * Le total annoncé est un plancher quand une source ne publie pas ses tailles :
 * sans borne, la barre irait au-delà de sa propre largeur.
 */
export function pourcentage(acquis: number, total: number): number {
  if (!(total > 0)) {
    return 0;
  }
  return Math.max(0, Math.min(100, (acquis / total) * 100));
}
