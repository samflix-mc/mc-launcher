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

/**
 * Où en est l'étape courante, entre zéro et un.
 *
 * **Deux mesures, et la seconde n'est pas un pis-aller.** Le poids sert quand
 * on le connaît : c'est le plus fidèle, puisque les fichiers n'ont pas tous la
 * même taille. Mais la résolution des mods ne descend presque rien — une
 * trentaine de secondes d'interrogation d'API pour quelques kilooctets — et
 * CurseForge ne publie pas les tailles sans clé : le total d'octets vaut alors
 * zéro, et la fraction restait à zéro pendant tout ce temps.
 *
 * Le compte de demandes réglées, lui, avance. Un pack de cinquante mods montre
 * donc une barre qui bouge cinquante fois plutôt qu'une barre immobile — que
 * rien ne distingue d'un plantage, et c'est exactement le reproche fait à
 * l'écran d'installation.
 */
export function fractionDuLot(
  octets: number,
  total: number,
  fichiers: number,
  fichiersTotal: number,
): number {
  if (total > 0) {
    return octets / total;
  }
  if (fichiersTotal > 0) {
    return fichiers / fichiersTotal;
  }
  return 0;
}

/**
 * L'avancement de toute l'installation, et non du seul lot en cours.
 *
 * Chaque étape annonce son propre lot, donc la barre du lot repasse par zéro
 * sept fois. Voir « 100 % » sept fois de suite est exactement ce qui fait
 * douter qu'il se passe quelque chose : on compte donc les étapes franchies,
 * et la fraction du lot ne fait qu'avancer à l'intérieur de la sienne.
 *
 * `etapes` est le nombre d'étapes du chemin qui comptent réellement — les
 * neuf de l'installation, pas « prêt » ni « lancé », qui ne sont pas du
 * travail.
 */
export function progressionGlobale(rang: number, fraction: number, etapes: number): number {
  if (!(etapes > 0) || rang < 0) {
    return 0;
  }
  const bornee = Math.max(0, Math.min(1, fraction));
  return Math.max(0, Math.min(100, ((rang + bornee) / etapes) * 100));
}

/**
 * Une couleur stable tirée d'un identifiant.
 *
 * Sert à donner une pastille reconnaissable à un joueur sans appeler de service
 * d'avatars : un UUID part chez un tiers, et ce launcher ne fait sortir que ce
 * qu'il doit. Deux comptes différents ont deux teintes différentes, et la même
 * à chaque lancement.
 */
export function teinte(identifiant: string): number {
  let somme = 0;
  for (const caractere of identifiant) {
    somme = (somme * 31 + caractere.charCodeAt(0)) % 360;
  }
  return somme;
}

/** Les deux premières lettres d'un pseudo, en capitales. */
export function initiales(pseudo: string): string {
  return pseudo.trim().slice(0, 2).toUpperCase() || '?';
}
