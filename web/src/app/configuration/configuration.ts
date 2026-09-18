import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';

import type { Dossier, Fond, ModeFenetre } from '../noyau/contrats';
import { Incidents } from '../noyau/incidents';
import { Pack } from '../noyau/pack';
import { Pont } from '../noyau/pont';
import { Reglages } from '../noyau/reglages';

/**
 * Les réglages, en cinq sections.
 *
 * ## La correspondance des sections, qui ne va pas de soi
 *
 * | Section affichée | Ce qu'elle règle |
 * |---|---|
 * | **Apparence** | `apparence` — la fenêtre du LAUNCHER |
 * | **Fenêtre** | `fenetre` — la fenêtre du JEU |
 * | **Vidéo** | `jeu` — les clés fusionnées dans `options.txt` |
 * | **Java** | rien de persisté : lu dans le VERROU |
 * | **Avancé** | `lanceur`, plus la vérification et les dossiers |
 *
 * « Apparence » et « Fenêtre » se confondent à la lecture : la première est la
 * nôtre, la seconde celle du jeu. C'est pour cela que les titres le disent.
 *
 * ## Java ne se règle pas
 *
 * Décision de Sam, et elle est juste : la majeure est une propriété du PACK,
 * écrite dans le verrou et vérifiée à chaque lancement. Offrir un choix
 * laisserait le poste du joueur contredire ce que le pack a figé, et le
 * serveur trancherait par une éjection qui ne nomme pas sa cause. La section
 * affiche donc, et n'offre rien.
 *
 * Elle lit `EtatDuPack.java`, qui vient du verrou publié : la valeur est donc
 * renseignée même quand rien n'est encore installé.
 */
@Component({
  selector: 'app-configuration',
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './configuration.html',
  styleUrl: './configuration.css',
})
export class Configuration {
  private readonly reglages = inject(Reglages);
  private readonly incidents = inject(Incidents);
  private readonly pont = inject(Pont);
  private readonly pack = inject(Pack);

  protected readonly vue = this.reglages.vue;
  protected readonly ecran = this.reglages.ecran;
  protected readonly etatDuPack = this.pack.etat;

  /** Le résultat de la dernière vérification, ou `null`. */
  protected readonly verification = signal<string[] | null>(null);
  protected readonly verificationEnCours = signal(false);

  protected readonly fonds: readonly { valeur: Fond; libelle: string }[] = [
    { valeur: 'spawn', libelle: 'Spawn' },
    { valeur: 'nether', libelle: 'Nether' },
    { valeur: 'fin', libelle: 'End' },
    { valeur: 'uni', libelle: 'Uni (sans image)' },
  ];

  protected readonly modes: readonly { valeur: ModeFenetre; libelle: string }[] = [
    { valeur: 'fenetree', libelle: 'Fenêtrée' },
    { valeur: 'maximisee', libelle: 'Maximisée' },
    { valeur: 'plein-ecran', libelle: 'Plein écran' },
  ];

  /**
   * La taille de l'écran, quand on la connaît.
   *
   * C'est la ZONE UTILE, panneaux du bureau déduits — et non la taille brute :
   * une fenêtre de la taille de l'écran passerait sous la barre des tâches.
   */
  protected readonly tailleEcran = computed(() => {
    const e = this.ecran();
    return e ? `${e.largeur} × ${e.hauteur}` : null;
  });

  protected readonly memoireGo = computed(() => {
    const mo = this.vue().lanceur.memoireMo;
    return mo === null ? null : Math.round((mo / 1024) * 10) / 10;
  });

  // --- Les changements -----------------------------------------------------
  //
  // Un `<select>` et un `<input type="range">` se lient par `[value]` et
  // `(input)`, et NON par `model()` : `model()` ne crée pas de liaison vers un
  // contrôle natif. Il sert là où un réglage devient un composant à lui, ce qui
  // n'est le cas d'aucun de ceux-ci.

  protected async changerFond(valeur: string): Promise<void> {
    const apparence = { ...this.vue().apparence, fond: valeur as Fond };
    await this.incidents.pendant(() => this.reglages.modifier({ apparence }));
  }

  protected async changerVoile(valeur: string): Promise<void> {
    const apparence = { ...this.vue().apparence, voile: Number(valeur) };
    await this.incidents.pendant(() => this.reglages.modifier({ apparence }));
  }

  protected async changerMode(valeur: string): Promise<void> {
    const fenetre = { ...this.vue().fenetre, mode: valeur as ModeFenetre };
    await this.incidents.pendant(() => this.reglages.modifier({ fenetre }));
  }

  protected async changerTaille(champ: 'largeur' | 'hauteur', valeur: string): Promise<void> {
    const fenetre = { ...this.vue().fenetre, [champ]: Number(valeur) };
    await this.incidents.pendant(() => this.reglages.modifier({ fenetre }));
  }

  protected async changerJeu(champ: string, valeur: string | boolean): Promise<void> {
    const jeu = {
      ...this.vue().jeu,
      [champ]: typeof valeur === 'boolean' ? valeur : Number(valeur),
    };
    await this.incidents.pendant(() => this.reglages.modifier({ jeu }));
  }

  protected async changerMemoire(valeur: string): Promise<void> {
    const lanceur = { ...this.vue().lanceur, memoireMo: Math.round(Number(valeur) * 1024) };
    await this.incidents.pendant(() => this.reglages.modifier({ lanceur }));
  }

  protected async changerReduction(valeur: boolean): Promise<void> {
    const lanceur = { ...this.vue().lanceur, reduireAuLancement: valeur };
    await this.incidents.pendant(() => this.reglages.modifier({ lanceur }));
  }

  // --- La section Avancé ---------------------------------------------------

  /**
   * « Vérifier les fichiers » — le geste qui a quitté l'écran principal.
   *
   * Il y était, il n'y est plus, et c'est ici qu'il réapparaît : sans cette
   * section, il n'aurait plus aucune porte d'entrée graphique.
   */
  protected async verifier(profond: boolean): Promise<void> {
    this.verificationEnCours.set(true);
    try {
      const problemes = await this.incidents.pendant(() => this.pont.verifierLesFichiers(profond));
      this.verification.set(problemes ?? null);
    } finally {
      this.verificationEnCours.set(false);
    }
  }

  protected async ouvrirDossier(quoi: Dossier): Promise<void> {
    await this.incidents.pendant(() => this.pont.ouvrirDossier(quoi));
  }
}
