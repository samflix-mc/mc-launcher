import { Component, computed, inject, signal, OnDestroy } from '@angular/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

import * as format from './format';
import {
  Launcher,
  messageDErreur,
  teteDuJoueur,
  type Marque,
  type Avancement,
  type CodeAppareil,
  type Compte,
  type EtapeVue,
  type Installation,
  type Phase,
} from './launcher';

/** Où en est une phase du chemin, du point de vue de l'affichage. */
type Etat = 'faite' | 'en-cours' | 'a-venir';

/**
 * Les phases qui sont un état et non un travail.
 *
 * Elles closent le chemin et ne comptent pas dans la progression : « prêt à
 * jouer » n'est pas une étape qu'on exécute, c'est le résultat des autres.
 */
const ABOUTISSEMENTS: readonly Phase[] = ['pret', 'lancement'];

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

  protected readonly marque = signal<Marque>({ nom: 'launcher', sceau: '??' });
  protected readonly chemin = signal<EtapeVue[]>([]);
  protected readonly compte = signal<Compte | null>(null);
  protected readonly code = signal<CodeAppareil | null>(null);
  protected readonly avancement = signal<Avancement | null>(null);
  protected readonly installation = signal<Installation | null>(null);
  protected readonly erreur = signal<string | null>(null);
  protected readonly message = signal<string | null>(null);

  /** Vrai tant qu'un appel à Rust est en cours : les boutons attendent. */
  protected readonly occupe = signal(false);
  /** Vrai pendant l'installation seulement : la barre remplace le bouton. */
  protected readonly installe = signal(false);
  /** Vrai pendant que le jeu tourne. */
  protected readonly joue = signal(false);

  /** Le pack est posé : le bouton « Jouer » s'allume. */
  protected readonly pret = computed(() => this.installation() !== null);

  /** Le compte est connecté et possède le jeu. */
  protected readonly jouable = computed(() => this.compte()?.possedeLeJeu === true);

  /** Les mods que la résolution n'a pas trouvés, s'il y en a. */
  protected readonly introuvables = computed(() => this.installation()?.introuvables ?? []);

  /** Ce par quoi l'installation s'écarte du verrou publié. */
  protected readonly ecarts = computed(() => this.installation()?.ecarts ?? []);

  /** Combien d'étapes comptent réellement dans la progression. */
  private readonly etapesUtiles = computed(
    () => this.chemin().filter((etape) => !ABOUTISSEMENTS.includes(etape.phase)).length,
  );

  /**
   * Le chemin annoté de l'état de chaque phase.
   *
   * Calculé à partir du rang plutôt que d'une liste tenue à jour : la phase
   * reçue suffit à savoir ce qui est derrière et ce qui reste, et rien ne peut
   * se désynchroniser.
   */
  protected readonly etapes = computed(() => {
    const vu = this.avancement();
    const rang = this.rangDe(vu?.phase);

    // Une phase « achevée » est derrière nous, pas en cours : « prêt à jouer »
    // ne doit pas clignoter comme s'il restait à l'attendre. Et tant que rien
    // n'a commencé, un compte connecté vaut les deux premières étapes.
    const enCours = vu && !vu.achevee ? rang : -1;
    const dernierFait = vu ? (vu.achevee ? rang : rang - 1) : this.rangSiConnecte();

    return this.chemin().map((etape) => ({
      ...etape,
      etat: (etape.rang <= dernierFait
        ? 'faite'
        : etape.rang === enCours
          ? 'en-cours'
          : 'a-venir') as Etat,
    }));
  });

  /** La progression de toute l'installation, pas du seul lot en cours. */
  protected readonly progression = computed(() => {
    const vu = this.avancement();
    if (!vu) {
      return 0;
    }
    const fraction = vu.total > 0 ? vu.octets / vu.total : 0;
    return format.progressionGlobale(this.rangDe(vu.phase), fraction, this.etapesUtiles());
  });

  /** L'étape courante, telle qu'on l'écrit : « 4 / 9 · Mods ». */
  protected readonly etapeCourante = computed(() => {
    const vu = this.avancement();
    if (!vu || vu.achevee) {
      return null;
    }
    const etape = this.chemin().find((candidate) => candidate.phase === vu.phase);
    return etape ? { numero: etape.rang + 1, total: this.etapesUtiles(), libelle: etape.libelle } : null;
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
        await this.launcher.surCodeAppareil((code) => this.code.set(code)),
      );
      this.marque.set(await this.launcher.marque());
      this.chemin.set(await this.launcher.chemin());
      this.installation.set(await this.launcher.installation());
      this.compte.set(await this.launcher.statut());
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    } finally {
      this.occupe.set(false);
    }
  }

  protected async connecter(): Promise<void> {
    await this.pendant(async () => {
      this.compte.set(await this.launcher.connexion());
      this.code.set(null);
    });
  }

  protected async deconnecter(): Promise<void> {
    await this.pendant(async () => {
      await this.launcher.deconnexion();
      this.compte.set(null);
      this.installation.set(null);
      this.avancement.set(null);
    });
  }

  protected async installer(): Promise<void> {
    this.installe.set(true);
    await this.pendant(async () => {
      this.installation.set(await this.launcher.installer());
    });
    this.installe.set(false);
  }

  protected async jouer(): Promise<void> {
    this.joue.set(true);
    await this.pendant(async () => {
      // Ne rend la main qu'à la fin de la partie.
      this.message.set(await this.launcher.lancerJeu());
    });
    this.joue.set(false);
  }

  protected async ouvrirLaPage(url: string): Promise<void> {
    try {
      await this.launcher.ouvrirPage(url);
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    }
  }

  /** Ferme l'erreur affichée. */
  protected fermerErreur(): void {
    this.erreur.set(null);
  }

  protected async copierErreur(): Promise<void> {
    const texte = this.erreur();
    if (texte) {
      // Sans `catch`, un presse-papiers refusé ferait une promesse rejetée
      // dans le vide, avec l'erreur d'origine remplacée dans la fenêtre.
      await navigator.clipboard.writeText(texte).catch(() => {});
    }
  }

  /** La progression arrondie, telle qu'on l'écrit. */
  protected pourcent(): number {
    return Math.round(this.progression());
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

  protected teinte(identifiant: string): number {
    return format.teinte(identifiant);
  }

  /** L'adresse du rendu de tête, pour l'attribut `src`. */
  protected tete(uuid: string): string {
    return teteDuJoueur(uuid);
  }

  /**
   * Le rendu n'a pas pu être chargé — hors ligne, service indisponible.
   *
   * On retombe sur la pastille de couleur plutôt que de laisser l'icône
   * d'image cassée du navigateur, qui est ce qu'on remarque le plus.
   */
  protected readonly teteIndisponible = signal(false);

  protected teteEnEchec(): void {
    this.teteIndisponible.set(true);
  }

  protected initiales(pseudo: string): string {
    return format.initiales(pseudo);
  }

  /**
   * Enveloppe un appel : occupe les boutons, capte l'erreur, libère à la fin.
   *
   * Chaque commande faisait la même séquence, avec chaque fois une chance
   * d'oublier le `finally` — et un bouton éteint pour le reste de la session.
   */
  private async pendant(action: () => Promise<void>): Promise<void> {
    this.erreur.set(null);
    this.message.set(null);
    this.occupe.set(true);
    try {
      await action();
    } catch (cause) {
      this.erreur.set(messageDErreur(cause));
    } finally {
      this.occupe.set(false);
    }
  }

  /** Le rang d'une phase dans le chemin, ou -1 si elle n'y est pas. */
  private rangDe(phase: Phase | undefined): number {
    return this.chemin().find((etape) => etape.phase === phase)?.rang ?? -1;
  }

  /**
   * Ce qui est acquis avant que rien n'ait commencé.
   *
   * Un compte connecté vaut la connexion et la licence : les laisser grises
   * ferait croire qu'il reste à les faire.
   */
  private rangSiConnecte(): number {
    return this.compte() ? this.rangDe('licence') : -1;
  }
}
