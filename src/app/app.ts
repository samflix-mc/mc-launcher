import { Component, computed, inject, signal, OnDestroy } from '@angular/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

import * as format from './format';
import {
  Launcher,
  messageDErreur,
  type Avancement,
  type CodeAppareil,
  type Compte,
  type EtapeVue,
  type Installation,
  type Phase,
} from './launcher';

/** Où en est une phase du chemin, du point de vue de l'affichage. */
type Etat = 'faite' | 'en-cours' | 'a-venir';

// Sans `styleUrl` : la mise en forme est entièrement dans `src/styles.css`.
// Angular injecterait les styles d'un composant à l'exécution, et le CSP à
// nonce que Tauri pose en production les rejette — le fichier de styles dit
// pourquoi en détail.
@Component({
  selector: 'app-root',
  imports: [],
  templateUrl: './app.html',
})
export class App implements OnDestroy {
  private readonly launcher = inject(Launcher);

  /** Faux hors de la fenêtre Tauri : l'écran le dit plutôt que d'échouer. */
  protected readonly disponible = this.launcher.disponible;

  protected readonly chemin = signal<EtapeVue[]>([]);
  protected readonly compte = signal<Compte | null>(null);
  protected readonly code = signal<CodeAppareil | null>(null);
  protected readonly avancement = signal<Avancement | null>(null);
  protected readonly installation = signal<Installation | null>(null);
  protected readonly erreur = signal<string | null>(null);
  protected readonly message = signal<string | null>(null);

  /** Vrai tant qu'un appel à Rust est en cours : les boutons attendent. */
  protected readonly occupe = signal(false);

  /** Le pack est posé : le bouton « Jouer » s'allume. */
  protected readonly pret = computed(() => this.installation() !== null);

  /** Le compte est connecté et possède le jeu. */
  protected readonly jouable = computed(() => this.compte()?.possedeLeJeu === true);

  /**
   * Le chemin annoté de l'état de chaque phase.
   *
   * Calculé à partir du rang plutôt que d'une liste tenue à jour : la phase
   * reçue suffit à savoir ce qui est derrière et ce qui reste, et rien ne peut
   * se désynchroniser.
   */
  protected readonly etapes = computed(() => {
    const enCours = this.rangDe(this.avancement()?.phase);

    // Rien ne tourne encore, mais un compte est là : la connexion et la
    // licence sont derrière nous, et le chemin doit le montrer. Les laisser
    // grises ferait croire qu'il reste à les faire.
    const acquis = this.compte() ? this.rangDe('licence') : -1;
    const dernierFait = enCours >= 0 ? enCours - 1 : acquis;

    return this.chemin().map((etape) => ({
      ...etape,
      etat: (etape.rang <= dernierFait
        ? 'faite'
        : etape.rang === enCours
          ? 'en-cours'
          : 'a-venir') as Etat,
    }));
  });

  /** La barre, en pourcentage. */
  protected readonly progression = computed(() => {
    const avancement = this.avancement();
    return avancement ? format.pourcentage(avancement.octets, avancement.total) : 0;
  });

  private desabonnements: UnlistenFn[] = [];

  constructor() {
    if (this.disponible) {
      void this.ouvrir();
    }
  }

  ngOnDestroy(): void {
    this.desabonnements.forEach((arreter) => arreter());
  }

  /**
   * À l'ouverture : le chemin à dessiner, puis qui est connecté.
   *
   * L'abonnement à l'avancement est posé une fois pour toutes, et pas à chaque
   * installation : un événement émis entre deux abonnements serait perdu, et
   * la barre resterait figée jusqu'au suivant.
   */
  protected async ouvrir(): Promise<void> {
    this.occupe.set(true);
    try {
      this.desabonnements.push(
        await this.launcher.surAvancement((avancement) => this.avancement.set(avancement)),
      );
      this.chemin.set(await this.launcher.chemin());
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
    this.desabonnements.push(
      await this.launcher.surCodeAppareil((code) => this.code.set(code)),
    );

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
      this.installation.set(null);
      this.avancement.set(null);
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    } finally {
      this.occupe.set(false);
    }
  }

  protected async installer(): Promise<void> {
    this.reinitialiser();
    this.occupe.set(true);
    try {
      this.installation.set(await this.launcher.installer());
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    } finally {
      this.occupe.set(false);
    }
  }

  protected async jouer(): Promise<void> {
    this.reinitialiser();
    this.occupe.set(true);
    try {
      // Ne rend la main qu'à la fin de la partie : le bouton reste éteint
      // pendant tout ce temps, ce qui est exactement ce qu'on veut.
      this.message.set(await this.launcher.lancerJeu());
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    } finally {
      this.occupe.set(false);
    }
  }

  protected async ouvrirLaPage(url: string): Promise<void> {
    try {
      await this.launcher.ouvrirPage(url);
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    }
  }

  protected octets(valeur: number): string {
    return format.octets(valeur);
  }

  protected debit(valeur: number): string {
    return format.debit(valeur);
  }

  protected duree(valeur: number): string {
    return format.duree(valeur);
  }

  /** La phase en cours travaille-t-elle sur des fichiers ? */
  protected telecharge(phase: Phase): boolean {
    return phase !== 'pret' && phase !== 'lancement' && (this.avancement()?.total ?? 0) > 0;
  }

  /** Le rang d'une phase dans le chemin, ou -1 si elle n'y est pas. */
  private rangDe(phase: Phase | undefined): number {
    return this.chemin().find((etape) => etape.phase === phase)?.rang ?? -1;
  }

  private reinitialiser(): void {
    this.erreur.set(null);
    this.message.set(null);
    this.code.set(null);
  }
}
