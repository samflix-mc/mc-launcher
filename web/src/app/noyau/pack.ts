import { Injectable, computed, inject, signal } from '@angular/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

import type { Avancement, CompteRendu, EtapeVue, EtatDuPack, Phase } from './contrats';
import { Pont } from './pont';
import * as format from './format';

/**
 * Les phases qui sont un ÉTAT et non un travail.
 *
 * Elles closent le chemin et ne comptent pas dans la progression : « prêt à
 * jouer » n'est pas une étape qu'on exécute, c'est le résultat des autres.
 */
const ABOUTISSEMENTS: readonly Phase[] = ['pret', 'lancement'];

/** Où en est une phase du chemin, du point de vue de l'affichage. */
export type Etat = 'faite' | 'en-cours' | 'a-venir';

/** Ce que le bouton affiche, activité comprise. */
export type Bouton =
  | 'inconnu' // on regarde encore
  | 'installer'
  | 'jouer'
  | 'occupe' // on rattrape
  | 'en-partie';

/**
 * L'état du pack, et ce que le bouton en dit.
 *
 * ## Pourquoi l'état du bouton n'a pas trois valeurs mais cinq
 *
 * L'`Action` que Rust rend dit ce que LE DISQUE impose : installer ou jouer.
 * Elle ne dit rien de l'ACTIVITÉ — une installation en cours, une partie qui
 * tourne — parce que l'activité ne se lit pas sur un disque, elle s'observe.
 *
 * Confondre les deux ferait répondre « une installation est déjà en cours » à
 * quelqu'un qui clique pendant sa partie, laquelle est le plus long état de la
 * session.
 *
 * Et « inconnu » est une cinquième valeur nécessaire : entre l'affichage de
 * Spawn et la réponse d'`etat_du_pack()`, un défaut « Installer » produirait un
 * clignotement INSTALLER → JOUER à chaque démarrage, sur le seul élément de
 * l'écran qui compte.
 */
@Injectable({ providedIn: 'root' })
export class Pack {
  private readonly pont = inject(Pont);

  readonly etat = signal<EtatDuPack | null>(null);
  readonly chemin = signal<EtapeVue[]>([]);
  readonly avancement = signal<Avancement | null>(null);
  /** Ce que le dernier geste — installation ou partie — a laissé. */
  readonly dernierCompteRendu = signal<CompteRendu | null>(null);

  /** Vrai pendant que le rattrapage ou la partie tournent. */
  readonly occupe = signal(false);

  /**
   * Vrai pendant que le jeu tourne, ce qui est plus long que tout le reste.
   *
   * DÉRIVÉ de l'avancement et non posé à la main : un signal qu'on doit penser
   * à remettre à zéro se retrouve un jour bloqué à `true`, et le bouton reste
   * « En jeu » pour le reste de la session. Ici, la cinématique quitte
   * « lancement » pour « pret » à la fin de la partie, et le calcul suit.
   */
  readonly enPartie = computed(() => this.avancement()?.phase === 'lancement');

  readonly bouton = computed<Bouton>(() => {
    if (this.enPartie()) {
      return 'en-partie';
    }
    if (this.occupe()) {
      return 'occupe';
    }
    const etat = this.etat();
    if (!etat) {
      return 'inconnu';
    }
    return etat.action === 'installer' ? 'installer' : 'jouer';
  });

  /** Combien d'étapes comptent réellement dans la progression. */
  private readonly etapesUtiles = computed(
    () => this.chemin().filter((etape) => !ABOUTISSEMENTS.includes(etape.phase)).length,
  );

  /**
   * Le chemin annoté de l'état de chaque phase.
   *
   * Calculé à partir du RANG plutôt que d'une liste tenue à jour : la phase
   * reçue suffit à savoir ce qui est derrière et ce qui reste, et rien ne peut
   * se désynchroniser.
   */
  readonly etapes = computed(() => {
    const vu = this.avancement();
    const rang = this.rangDe(vu?.phase);

    // Une phase « achevée » est derrière nous, pas en cours : « prêt à jouer »
    // ne doit pas clignoter comme s'il restait à l'attendre.
    const enCours = vu && !vu.achevee ? rang : -1;
    const dernierFait = vu ? (vu.achevee ? rang : rang - 1) : -1;

    return this.chemin().map((etape) => ({
      ...etape,
      etat: (etape.rang <= dernierFait
        ? 'faite'
        : etape.rang === enCours
          ? 'en-cours'
          : 'a-venir') as Etat,
    }));
  });

  /** La progression de TOUTE l'installation, pas du seul lot en cours. */
  readonly progression = computed(() => {
    const vu = this.avancement();
    if (!vu) {
      return 0;
    }
    const fraction = vu.total > 0 ? vu.octets / vu.total : 0;
    return format.progressionGlobale(this.rangDe(vu.phase), fraction, this.etapesUtiles());
  });

  /** L'étape courante, telle qu'on l'écrit : « 4 / 9 · Mods ». */
  readonly etapeCourante = computed(() => {
    const vu = this.avancement();
    if (!vu || vu.achevee) {
      return null;
    }
    const etape = this.chemin().find((candidate) => candidate.phase === vu.phase);
    return etape
      ? { numero: etape.rang + 1, total: this.etapesUtiles(), libelle: etape.libelle }
      : null;
  });

  private desabonner: UnlistenFn | null = null;

  /**
   * À l'ouverture : le chemin à dessiner, l'abonnement, puis l'état du pack.
   *
   * L'abonnement à l'avancement est posé UNE fois pour toutes, et non à chaque
   * partie : un événement émis entre deux abonnements serait perdu, et la
   * barre resterait figée jusqu'au suivant.
   */
  async ouvrir(): Promise<void> {
    if (!this.pont.disponible) {
      return;
    }
    if (!this.desabonner) {
      this.desabonner = await this.pont.surAvancement((a) => this.avancement.set(a));
      this.chemin.set(await this.pont.chemin());
    }
    await this.rafraichir();
  }

  /** Redemande l'état du pack. Quelques dizaines de kio, aucun disque touché. */
  async rafraichir(): Promise<void> {
    if (!this.pont.disponible) {
      return;
    }
    this.etat.set(await this.pont.etatDuPack());
  }

  /**
   * Pose ce qu'il y a à poser, et s'arrête là.
   *
   * Le geste du bouton quand quelque chose manque ou a bougé. Il ne lance pas
   * le jeu : c'est un second clic, sur un bouton qui dira alors « Jouer ».
   */
  async installer(): Promise<CompteRendu> {
    return this.pendantLeGeste(() => this.pont.installer());
  }

  /**
   * Vérifie, rattrape s'il le faut, puis lance la partie.
   *
   * `occupe` couvre tout l'appel ; `enPartie` ne s'allume qu'une fois le
   * rattrapage fini — c'est-à-dire quand la cinématique atteint « lancement ».
   * Les distinguer est ce qui permet au bouton de dire « Installation… » puis
   * « En jeu » plutôt qu'un « Occupé » indistinct pendant vingt minutes.
   */
  async jouer(): Promise<CompteRendu> {
    return this.pendantLeGeste(() => this.pont.jouer());
  }

  /**
   * Ce que les deux gestes ont en commun.
   *
   * Extrait parce que les trois temps — lever `occupe`, retenir le compte
   * rendu, relire le disque — doivent être les mêmes : les recopier laisserait
   * un jour l'un des deux oublier de rafraîchir, et le bouton garderait le
   * libellé d'avant l'installation qu'il vient de faire.
   */
  private async pendantLeGeste(geste: () => Promise<CompteRendu>): Promise<CompteRendu> {
    this.occupe.set(true);
    try {
      const rendu = await geste();
      this.dernierCompteRendu.set(rendu);
      // Le disque a changé : l'état d'avant ne vaut plus.
      await this.rafraichir();
      return rendu;
    } finally {
      this.occupe.set(false);
    }
  }

  /** Le rang d'une phase dans le chemin, ou -1 si elle n'y est pas. */
  private rangDe(phase: Phase | undefined): number {
    return this.chemin().find((etape) => etape.phase === phase)?.rang ?? -1;
  }
}
