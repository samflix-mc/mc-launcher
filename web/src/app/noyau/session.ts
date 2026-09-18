import { Injectable, computed, inject, signal } from '@angular/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

import type { CodeAppareil, Compte } from './contrats';
import { Pont } from './pont';

/**
 * Qui est connecté, et comment on le devient.
 *
 * ## Le prédicat unique
 *
 * [`jouable`] est LE prédicat dont dépendent les deux gardes du routeur. Un
 * seul, et c'est délibéré : deux prédicats différents feraient rebondir sans
 * fin un compte connecté SANS LICENCE entre `/connexion` et `/spawn` — l'un le
 * trouverait connecté, l'autre pas jouable, et chacun renverrait vers l'autre.
 *
 * Ce cas n'est pas théorique : c'est celui d'un compte Microsoft valide qui
 * n'a jamais acheté Minecraft, et l'écran doit le dire plutôt que de tourner.
 */
@Injectable({ providedIn: 'root' })
export class Session {
  private readonly pont = inject(Pont);

  /** Le compte, ou `null`. `undefined` tant qu'on n'a pas encore demandé. */
  readonly compte = signal<Compte | null | undefined>(undefined);

  /** Le code à saisir chez Microsoft, pendant l'attente. */
  readonly code = signal<CodeAppareil | null>(null);

  /** Un appel est-il en cours ? */
  readonly occupe = signal(false);

  /** A-t-on déjà interrogé Rust ? */
  readonly connue = computed(() => this.compte() !== undefined);

  /** Connecté ET propriétaire du jeu. LE prédicat des deux gardes. */
  readonly jouable = computed(() => this.compte()?.possedeLeJeu === true);

  /** Connecté, mais sans licence : un état à afficher, pas à rediriger. */
  readonly sansLicence = computed(() => this.compte()?.possedeLeJeu === false);

  private desabonner: UnlistenFn | null = null;

  /**
   * Interroge Rust, une fois.
   *
   * L'accès rafraîchit paresseusement les jetons côté Rust : sans cet appel,
   * le lancement suivant repartirait du jeton périmé et redemanderait un code
   * pour rien.
   */
  async ouvrir(): Promise<void> {
    if (!this.pont.disponible) {
      this.compte.set(null);
      return;
    }
    if (!this.desabonner) {
      this.desabonner = await this.pont.surCodeAppareil((code) => this.code.set(code));
    }
    this.compte.set(await this.pont.statut());
  }

  /**
   * Ouvre une session Microsoft.
   *
   * Ne rend la main qu'une fois le joueur passé chez Microsoft — ou le code
   * expiré. Le code, lui, est arrivé par événement pendant l'attente.
   */
  async connecter(): Promise<void> {
    this.occupe.set(true);
    try {
      this.compte.set(await this.pont.connexion());
      this.code.set(null);
    } finally {
      this.occupe.set(false);
    }
  }

  async deconnecter(): Promise<void> {
    this.occupe.set(true);
    try {
      await this.pont.deconnexion();
      this.compte.set(null);
      this.code.set(null);
    } finally {
      this.occupe.set(false);
    }
  }
}
