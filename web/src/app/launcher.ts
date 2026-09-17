import { Injectable } from '@angular/core';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';

/** Le compte connecté, tel que `commandes.rs` le sérialise. */
export interface Compte {
  readonly pseudo: string;
  readonly uuid: string;
  readonly possedeLeJeu: boolean;
}

/** Ce que le joueur doit saisir chez Microsoft. */
export interface CodeAppareil {
  readonly code: string;
  readonly url: string;
  readonly urlDirecte: string;
}

/**
 * Les identifiants de phase, tels que `phase.rs` les sérialise.
 *
 * Écrits des deux côtés : les renommer d'un seul laisse une fenêtre qui
 * n'éclaire plus la bonne ligne, sans qu'aucune compilation ne s'en plaigne.
 */
export type Phase =
  | 'connexion'
  | 'licence'
  | 'pack'
  | 'chargeur'
  | 'minecraft'
  | 'java'
  | 'neo-forge'
  | 'mods'
  | 'verrou'
  | 'pret'
  | 'lancement';

/** Une phase du chemin, avec de quoi la dessiner. */
export interface EtapeVue {
  readonly phase: Phase;
  readonly libelle: string;
  readonly rang: number;
}

/** L'état de l'installation, cinq fois par seconde. */
export interface Avancement {
  readonly phase: Phase;
  /** La phase est-elle un état atteint plutôt qu'un travail en cours ? */
  readonly achevee: boolean;
  readonly note: string | null;
  readonly fichier: string | null;
  readonly octets: number;
  readonly total: number;
  readonly fichiers: number;
  readonly fichiersTotal: number;
  /** Un téléchargement est-il réellement en cours ? */
  readonly actif: boolean;
  readonly debit: number;
  readonly restant: number | null;
}

/** Ce qu'une installation a posé. */
export interface Installation {
  readonly instance: string;
  readonly minecraft: string;
  readonly neoforge: string;
  readonly java: string;
  readonly mods: number;
  /** Les mods que la résolution n'a pas trouvés. */
  readonly introuvables: readonly string[];
  readonly horsLigne: boolean;
}

const EVENEMENT_CODE = 'auth://code';
const EVENEMENT_AVANCEMENT = 'cinematique://avancement';

/**
 * Le seul endroit qui parle à Rust.
 *
 * Tout passe par ici pour une raison simple : hors de la fenêtre Tauri —
 * `ng serve` seul, une suite de tests — `invoke` n'existe pas. Le composant
 * n'a pas à le savoir, il lit `disponible`.
 */
@Injectable({ providedIn: 'root' })
export class Launcher {
  /** Vrai dans la fenêtre Tauri, faux dans un navigateur ordinaire. */
  readonly disponible = isTauri();

  /** Le chemin complet, demandé une fois à l'ouverture. */
  chemin(): Promise<EtapeVue[]> {
    return invoke<EtapeVue[]>('chemin');
  }

  /** Le compte déjà connecté sur cette machine, ou `null`. */
  statut(): Promise<Compte | null> {
    return invoke<Compte | null>('statut');
  }

  /** Ouvre une session Microsoft. Ne rend la main qu'une fois le code validé. */
  connexion(): Promise<Compte> {
    return invoke<Compte>('connexion');
  }

  /** Oublie la session, dans le trousseau comme dans le fichier. */
  deconnexion(): Promise<void> {
    return invoke<void>('deconnexion');
  }

  /** Ce que la dernière installation de cette session a posé, ou `null`. */
  installation(): Promise<Installation | null> {
    return invoke<Installation | null>('installation');
  }

  /**
   * Installe le pack. Plusieurs minutes.
   *
   * L'avancement ne passe pas par la promesse : il arrive par événement,
   * pendant tout ce temps.
   */
  installer(): Promise<Installation> {
    return invoke<Installation>('installer');
  }

  /** Lance le jeu, et rend la main quand la partie se termine. */
  lancerJeu(): Promise<string> {
    return invoke<string>('lancer_jeu');
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

  /** S'abonne à l'avancement de la cinématique. */
  surAvancement(recevoir: (avancement: Avancement) => void): Promise<UnlistenFn> {
    return listen<Avancement>(EVENEMENT_AVANCEMENT, (evenement) => recevoir(evenement.payload));
  }

  /** Rouvre la page Microsoft — Rust l'a déjà tenté, ceci est le recours. */
  ouvrirPage(url: string): Promise<void> {
    return openUrl(url);
  }
}

/**
 * Le message d'une erreur venue de Rust.
 *
 * `Erreur` se sérialise en chaîne ; ce qui remonte autrement vient du pont
 * lui-même, et on ne veut pas afficher « [object Object] » à un joueur.
 */
export function messageDErreur(cause: unknown): string {
  if (typeof cause === 'string') {
    return cause;
  }
  if (cause instanceof Error) {
    return cause.message;
  }
  // `String({})` donnerait « [object Object] », qui n'apprend rien à personne.
  try {
    return JSON.stringify(cause) ?? String(cause);
  } catch {
    return String(cause);
  }
}
