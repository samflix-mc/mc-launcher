import { Component, inject, signal, OnDestroy } from '@angular/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

import { Launcher, messageDErreur, type CodeAppareil, type Compte } from './launcher';

@Component({
  selector: 'app-root',
  imports: [],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App implements OnDestroy {
  private readonly launcher = inject(Launcher);

  /** Faux hors de la fenêtre Tauri : l'écran le dit plutôt que d'échouer. */
  protected readonly disponible = this.launcher.disponible;

  protected readonly compte = signal<Compte | null>(null);
  protected readonly code = signal<CodeAppareil | null>(null);
  protected readonly erreur = signal<string | null>(null);
  protected readonly message = signal<string | null>(null);

  /** Vrai tant qu'un appel à Rust est en cours : les boutons attendent. */
  protected readonly occupe = signal(false);

  private desabonner: UnlistenFn | null = null;

  constructor() {
    if (this.disponible) {
      void this.reprendreLaSession();
    }
  }

  ngOnDestroy(): void {
    this.desabonner?.();
  }

  /**
   * À l'ouverture : y a-t-il déjà quelqu'un de connecté ?
   *
   * L'appel rafraîchit les jetons au passage, et peut donc échouer sur une
   * session trop vieille. Ce n'est pas une panne — c'est un message, et le
   * bouton de connexion reste là.
   */
  protected async reprendreLaSession(): Promise<void> {
    this.occupe.set(true);
    try {
      this.compte.set(await this.launcher.statut());
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    } finally {
      this.occupe.set(false);
    }
  }

  protected async connecter(): Promise<void> {
    this.reinitialiser();
    this.occupe.set(true);

    // L'abonnement d'abord : `connexion()` n'attend pas, le code peut arriver
    // avant que la promesse d'écoute ne soit résolue.
    this.desabonner?.();
    this.desabonner = await this.launcher.surCodeAppareil((code) => this.code.set(code));

    try {
      this.compte.set(await this.launcher.connexion());
      this.code.set(null);
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    } finally {
      this.occupe.set(false);
    }
  }

  protected async deconnecter(): Promise<void> {
    this.reinitialiser();
    this.occupe.set(true);
    try {
      await this.launcher.deconnexion();
      this.compte.set(null);
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    } finally {
      this.occupe.set(false);
    }
  }

  protected async jouer(): Promise<void> {
    this.reinitialiser();
    try {
      this.message.set(await this.launcher.lancerJeu());
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    }
  }

  protected async ouvrirLaPage(url: string): Promise<void> {
    try {
      await this.launcher.ouvrirPage(url);
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    }
  }

  private reinitialiser(): void {
    this.erreur.set(null);
    this.message.set(null);
    this.code.set(null);
  }
}
