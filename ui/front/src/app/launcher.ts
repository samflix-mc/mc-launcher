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
 * Le nom de l'événement est écrit des deux côtés — `EVENEMENT_CODE` en Rust.
 * Le changer ici seul laisse une fenêtre qui attend un code déjà émis.
 */
const EVENEMENT_CODE = 'auth://code';

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

  /** Pour l'instant : rend le message expliquant que ce n'est pas branché. */
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
