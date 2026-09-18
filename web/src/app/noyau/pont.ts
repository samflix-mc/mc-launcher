import { Injectable } from '@angular/core';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';

import type {
  Avancement,
  CodeAppareil,
  Compte,
  Dossier,
  Ecran,
  EtapeVue,
  EtatDuPack,
  Fil,
  Marque,
  Partie,
  Reglages,
} from './contrats';

const EVENEMENT_CODE = 'auth://code';
const EVENEMENT_AVANCEMENT = 'cinematique://avancement';

/**
 * Le SEUL endroit qui parle à Rust.
 *
 * Tout passe par ici pour une raison précise : hors de la fenêtre Tauri —
 * `ng serve` seul, une suite de tests — `invoke` n'existe pas. Aucun composant
 * n'a à le savoir ; ils lisent `disponible`, ou passent par un service de
 * `noyau/` qui s'en charge.
 *
 * Ce service ne contient AUCUNE règle. Il appelle et il rend. Tout ce qui
 * décide vit soit dans un service du noyau, soit — de préférence — en Rust,
 * où la mutation l'atteint.
 */
@Injectable({ providedIn: 'root' })
export class Pont {
  /** Vrai dans la fenêtre Tauri, faux dans un navigateur ordinaire. */
  readonly disponible = isTauri();

  // --- Le démarrage --------------------------------------------------------

  /**
   * Dit à Rust que le front a fini de se rendre.
   *
   * C'est ce qui referme l'écran de démarrage — une seconde fenêtre, sans
   * script, ouverte pendant qu'Angular se charge — et qui montre la fenêtre
   * principale, jusque-là cachée.
   *
   * Rust ne peut pas le deviner : il sait quand une fenêtre EXISTE, pas quand
   * son contenu est peint. Le front est le seul à le savoir.
   */
  frontPret(): Promise<void> {
    return invoke<void>('front_pret');
  }

  // --- Identité du launcher ------------------------------------------------

  marque(): Promise<Marque> {
    return invoke<Marque>('marque');
  }

  chemin(): Promise<EtapeVue[]> {
    return invoke<EtapeVue[]>('chemin');
  }

  // --- Session -------------------------------------------------------------

  statut(): Promise<Compte | null> {
    return invoke<Compte | null>('statut');
  }

  /** Ouvre une session Microsoft. Ne rend la main qu'une fois le code validé. */
  connexion(): Promise<Compte> {
    return invoke<Compte>('connexion');
  }

  deconnexion(): Promise<void> {
    return invoke<void>('deconnexion');
  }

  /**
   * S'abonne au code d'appareil.
   *
   * Il arrive par événement et non en valeur de retour : `connexion()` attend
   * que le joueur ait autorisé, et le code doit être affiché pendant cette
   * attente.
   */
  surCodeAppareil(recevoir: (code: CodeAppareil) => void): Promise<UnlistenFn> {
    return listen<CodeAppareil>(EVENEMENT_CODE, (evenement) => recevoir(evenement.payload));
  }

  // --- Le pack -------------------------------------------------------------

  /** Ce que le disque et le pack publié disent. Quelques dizaines de kio. */
  etatDuPack(): Promise<EtatDuPack> {
    return invoke<EtatDuPack>('etat_du_pack');
  }

  /**
   * LE bouton : vérifie, rattrape s'il le faut, puis lance la partie.
   *
   * Ne rend la main qu'à la fin de la partie. L'avancement arrive par
   * événement pendant tout ce temps.
   */
  jouer(): Promise<Partie> {
    return invoke<Partie>('jouer');
  }

  verifierLesFichiers(profond: boolean): Promise<string[]> {
    return invoke<string[]>('verifier_les_fichiers', { profond });
  }

  surAvancement(recevoir: (avancement: Avancement) => void): Promise<UnlistenFn> {
    return listen<Avancement>(EVENEMENT_AVANCEMENT, (evenement) => recevoir(evenement.payload));
  }

  // --- Les nouvelles -------------------------------------------------------

  nouvelles(): Promise<Fil> {
    return invoke<Fil>('nouvelles');
  }

  // --- Les réglages --------------------------------------------------------

  reglages(): Promise<Reglages> {
    return invoke<Reglages>('reglages');
  }

  /** Rend ce qui a été ÉCRIT, et non ce qu'on a envoyé. Voir `mc-reglages`. */
  enregistrerReglages(reglages: Reglages): Promise<Reglages> {
    return invoke<Reglages>('enregistrer_reglages', { reglages });
  }

  ecran(): Promise<Ecran | null> {
    return invoke<Ecran | null>('ecran');
  }

  ouvrirDossier(quoi: Dossier): Promise<void> {
    return invoke<void>('ouvrir_dossier', { quoi });
  }

  // --- Le système ----------------------------------------------------------

  /**
   * Ouvre une URL dans le navigateur DU SYSTÈME.
   *
   * Jamais dans la fenêtre : celle-ci est une origine privilégiée où `invoke`
   * est joignable, et le greffon de navigation de Rust refuserait de toute
   * façon. C'est le chemin de sortie de tout lien d'un billet.
   */
  ouvrirPage(url: string): Promise<void> {
    return openUrl(url);
  }
}

/**
 * La tête du joueur, rendue en trois dimensions par mc-heads.net.
 *
 * Un service tiers, et c'est un choix : l'UUID part chez lui. Il est public —
 * n'importe quel serveur où le joueur se connecte le connaît — et c'est le
 * prix d'un vrai rendu de skin plutôt qu'une pastille de couleur. Le CSP de
 * `tauri.conf.json` autorise ce domaine et lui seul.
 */
export function teteDuJoueur(uuid: string, taille = 96): string {
  return `https://mc-heads.net/head/${encodeURIComponent(uuid)}/${taille}`;
}

/**
 * Le message d'une erreur venue de Rust.
 *
 * `Erreur` se sérialise en chaîne ; ce qui remonte autrement vient du pont
 * lui-même, et l'on ne veut pas afficher « [object Object] » à un joueur.
 */
export function messageDErreur(cause: unknown): string {
  if (typeof cause === 'string') {
    return cause;
  }
  if (cause instanceof Error) {
    return cause.message;
  }
  try {
    return JSON.stringify(cause) ?? String(cause);
  } catch {
    return String(cause);
  }
}
