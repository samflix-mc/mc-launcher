import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import {
  FolderOpen,
  MemoryStick,
  Monitor,
  ShieldCheck,
  SlidersHorizontal,
  Terminal,
} from '../noyau/icones';
import type { Dossier, Fond, ModeFenetre } from '../noyau/contrats';
import { Incidents } from '../noyau/incidents';
import { Notifications } from '../noyau/notifications';
import { Pack } from '../noyau/pack';
import { Pont } from '../noyau/pont';
import { Reglages } from '../noyau/reglages';

/** Une entrée du rail de gauche. */
interface Groupe {
  readonly ancre: string;
  readonly libelle: string;
  readonly icone: LucideIconData;
}

/**
 * Le plancher du voile, recopié de `mc-reglages::bornes::VOILE_PLANCHER`.
 *
 * ## Pourquoi il est écrit ici aussi
 *
 * Rust borne de toute façon — c'est lui qui décide. Mais un curseur qui propose
 * une valeur que Rust remonterait rend un geste qui SAUTE : on tire, la valeur
 * revient. C'est exactement ce que Sam a décrit par « le slider du voile
 * déconne », à l'époque où le curseur descendait à 0,35 et Rust remontait à
 * 0,44.
 *
 * Il vaut zéro depuis que le contraste est tenu par l'épaisseur du verre et non
 * par le voile — voir `VOILE_PLANCHER` côté Rust et `--glass-epaisseur` dans
 * `styles.css`. Les deux nombres doivent rester égaux ; si le plancher bouge
 * d'un seul côté, le symptôme revient tel quel.
 */
const VOILE_PLANCHER = 0;

/** Le plafond du curseur de mémoire, en gigaoctets. Voir `bornes::MEMOIRE`. */
const MEMOIRE_MAX_GO = 64;

/**
 * Les réglages, en cinq groupes, avec le rail du design system.
 *
 * ## La correspondance des groupes, qui ne va pas de soi
 *
 * | Groupe affiché | Ce qu'il règle |
 * |---|---|
 * | **Apparence** | `apparence` — la fenêtre du LAUNCHER |
 * | **Fenêtre du jeu** | `fenetre` — la fenêtre du JEU |
 * | **Vidéo** | `jeu` — les clés fusionnées dans `options.txt` |
 * | **Java** | rien de persisté : lu dans le VERROU |
 * | **Avancé** | `lanceur`, plus la vérification et les dossiers |
 *
 * « Apparence » et « Fenêtre » se confondent à la lecture : la première est la
 * nôtre, la seconde celle du jeu. C'est pour cela que les titres le disent.
 *
 * ## Java ne se règle pas
 *
 * La majeure est une propriété du PACK, écrite dans son verrou et vérifiée à
 * chaque lancement. Offrir un choix laisserait le poste du joueur contredire ce
 * que le pack a figé, et le serveur trancherait par une éjection qui ne nomme
 * pas sa cause. Le groupe affiche, et n'offre rien.
 *
 * ## Une seule vérification
 *
 * Il y en avait deux — « rapide » et « complète ». La rapide ne comparait que
 * les tailles : elle disait « tout est en place » sur un fichier corrompu de la
 * bonne longueur, ce qui est le seul cas où l'on vérifie. Il n'en reste qu'une,
 * la vraie, et elle annonce qu'elle prend du temps.
 */
@Component({
  selector: 'app-configuration',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './configuration.html',
  styleUrl: './configuration.css',
})
export class Configuration {
  private readonly reglages = inject(Reglages);
  private readonly incidents = inject(Incidents);
  private readonly notifications = inject(Notifications);
  private readonly pont = inject(Pont);
  private readonly pack = inject(Pack);

  protected readonly vue = this.reglages.vue;
  protected readonly ecran = this.reglages.ecran;
  protected readonly etatDuPack = this.pack.etat;

  /** Le résultat de la dernière vérification, ou `null`. */
  protected readonly verification = signal<string[] | null>(null);
  protected readonly verificationEnCours = signal(false);

  protected readonly VOILE_PLANCHER = VOILE_PLANCHER;
  protected readonly MEMOIRE_MAX_GO = MEMOIRE_MAX_GO;

  protected readonly ShieldCheck = ShieldCheck;
  protected readonly FolderOpen = FolderOpen;
  protected readonly Terminal = Terminal;

  protected readonly groupes: readonly Groupe[] = [
    { ancre: 'apparence', libelle: 'Apparence', icone: SlidersHorizontal },
    { ancre: 'fenetre', libelle: 'Fenêtre du jeu', icone: Monitor },
    { ancre: 'video', libelle: 'Vidéo', icone: Monitor },
    { ancre: 'java', libelle: 'Java', icone: Terminal },
    { ancre: 'avance', libelle: 'Avancé', icone: MemoryStick },
  ];

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

  /** Le pourcentage de course d'un curseur — le design system le lit dans `--p`. */
  protected course(valeur: number, bas: number, haut: number): string {
    return `${((valeur - bas) / (haut - bas)) * 100}%`;
  }

  // --- Les changements -----------------------------------------------------
  //
  // Un `<select>` et un `<input type="range">` se lient par `[value]` et
  // `(input)`, et NON par `model()` : `model()` ne crée pas de liaison vers un
  // contrôle natif.
  //
  // `(input)` et non `(change)` sur les curseurs : `change` n'arrive qu'au
  // relâchement, si bien que le nombre affiché restait figé pendant qu'on
  // tirait. C'est la seconde moitié de « le slider déconne ».

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

  // --- Le groupe Avancé ----------------------------------------------------

  /**
   * « Vérifier les fichiers » — le geste qui a quitté l'écran principal.
   *
   * Il y était, il n'y est plus, et c'est ici qu'il réapparaît : sans ce
   * groupe, il n'aurait plus aucune porte d'entrée graphique.
   */
  protected async verifier(): Promise<void> {
    this.verificationEnCours.set(true);
    try {
      const problemes = await this.incidents.pendant(() => this.pont.verifierLesFichiers(true));
      this.verification.set(problemes ?? null);
      if (problemes) {
        this.notifications.signaler(
          problemes.length === 0 ? 'success' : 'warning',
          problemes.length === 0 ? 'Vérification terminée' : `${problemes.length} problème(s)`,
          problemes.length === 0 ? 'Tout est en place.' : 'Le prochain lancement les rattrapera.',
        );
      }
    } finally {
      this.verificationEnCours.set(false);
    }
  }

  protected async ouvrirDossier(quoi: Dossier): Promise<void> {
    await this.incidents.pendant(() => this.pont.ouvrirDossier(quoi));
  }
}
