import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  effect,
  inject,
  signal,
  viewChildren,
} from '@angular/core';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import {
  FolderOpen,
  MemoryStick,
  Monitor,
  ShieldCheck,
  SlidersHorizontal,
  Terminal,
} from '../noyau/icones';
import type { Dossier, Fond, ModeFenetre, Reglages as ReglagesVue } from '../noyau/contrats';
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

/** Le plafond du curseur de mémoire, en gigaoctets. Voir `bornes::MEMOIRE`. */
const MEMOIRE_MAX_GO = 64;

/**
 * Les réglages, en cinq groupes, avec le rail du design system.
 *
 * ## Le brouillon, et pourquoi il n'est pas un détail
 *
 * Les contrôles lisent un ÉTAT LOCAL, pas le service. `(input)` ne fait que
 * mettre à jour cet état ; c'est `(change)` — le relâchement — qui enregistre.
 *
 * La première version liait tout directement au service, et enregistrait sur
 * `(input)`. Tirer un curseur d'un bout à l'autre partait alors en trente à
 * cinquante allers-retours par seconde, chacun écrivant le fichier ; les
 * réponses revenaient dans le désordre, la plus ancienne écrasait la plus
 * récente, et le nombre affiché restait figé pendant qu'on tirait. C'est
 * exactement ce que Sam a décrit : « ça déplace le bouton, mais ça ne change
 * pas la valeur ».
 *
 * Le brouillon sépare les deux temps : ce qu'on VOIT suit le doigt sans passer
 * par le réseau, ce qu'on ÉCRIT part une fois, au relâchement. Et comme Rust
 * rend ce qu'il a réellement écrit — une valeur ramenée dans ses bornes l'est
 * visiblement — le brouillon se resynchronise sur sa réponse.
 *
 * ## Le rail ne met rien dans l'URL
 *
 * Les entrées étaient des ancres `#reglages-…`, et elles ne fonctionnaient pas :
 * du temps de `withHashLocation()`, le dièse appartenait au ROUTEUR, si bien
 * que cliquer écrivait une URL qu'il essayait de résoudre comme une route — la
 * navigation repartait vers `/spawn`.
 *
 * Le fragment a disparu depuis, mais les boutons restent, et pour une raison
 * qui lui survit : faire défiler dans une page n'est pas naviguer. Une ancre y
 * laisserait une entrée d'historique que le bouton « précédent » relirait comme
 * un changement de page. C'est d'ailleurs ce que le design system décrit — « les
 * liens du rail font défiler jusqu'au groupe et marquent celui qui est en vue ».
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
 * bonne longueur, ce qui est le seul cas où l'on vérifie.
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

  /**
   * Ce que les contrôles affichent — le brouillon.
   *
   * Initialisé depuis le service, et resynchronisé par l'`effect` ci-dessous à
   * chaque fois que celui-ci change : au chargement, et après chaque
   * enregistrement, puisque Rust rend ce qu'il a réellement écrit.
   */
  protected readonly vue = signal<ReglagesVue>(this.reglages.vue());

  protected readonly ecran = this.reglages.ecran;
  protected readonly etatDuPack = this.pack.etat;

  /** Le résultat de la dernière vérification, ou `null`. */
  protected readonly verification = signal<string[] | null>(null);
  protected readonly verificationEnCours = signal(false);

  /** Le groupe actuellement en vue, marqué dans le rail. */
  protected readonly groupeEnVue = signal<string>('apparence');

  private readonly sections = viewChildren<ElementRef<HTMLElement>>('section');

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

  constructor() {
    // Le brouillon suit le service, jamais l'inverse. C'est ce qui fait qu'une
    // valeur ramenée dans ses bornes par Rust se voit tout de suite.
    effect(() => this.vue.set(this.reglages.vue()));

    // Et l'écran suit le brouillon : tirer le curseur du voile doit éclaircir
    // l'image SOUS LE DOIGT, alors que l'écriture n'a lieu qu'au relâchement.
    // Sans cela, on règlerait un assombrissement à l'aveugle.
    effect(() => this.reglages.refleter(this.vue()));

    // Le rail marque le groupe en vue, y compris quand on défile à la molette
    // et non par un clic. Sans cela, il ne marquerait que le dernier cliqué —
    // c'est-à-dire qu'il mentirait dès le premier coup de molette.
    effect((suppression) => {
      const sections = this.sections();
      // `IntersectionObserver` manque à jsdom, où les suites tournent. Son
      // absence ne doit rien casser : le rail marque alors le dernier groupe
      // cliqué, ce qui est exactement son comportement d'avant l'observateur.
      if (sections.length === 0 || typeof IntersectionObserver === 'undefined') {
        return;
      }
      const observateur = new IntersectionObserver(
        (entrees) => {
          const vue = entrees
            .filter((entree) => entree.isIntersecting)
            .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
          const groupe = (vue?.target as HTMLElement | undefined)?.dataset['groupe'];
          if (groupe) {
            this.groupeEnVue.set(groupe);
          }
        },
        // Une bande au tiers haut de la zone : c'est là que l'œil se pose, et
        // c'est ce qui évite qu'un groupe à peine entré par le bas prenne la
        // marque alors qu'on lit encore le précédent.
        { rootMargin: '0px 0px -66% 0px', threshold: [0, 0.25, 0.5] },
      );
      for (const section of sections) {
        observateur.observe(section.nativeElement);
      }
      suppression(() => observateur.disconnect());
    });
  }

  /** Le pourcentage de course d'un curseur — le design system le lit dans `--p`. */
  protected course(valeur: number, bas: number, haut: number): string {
    return `${((valeur - bas) / (haut - bas)) * 100}%`;
  }

  /**
   * Fait défiler jusqu'à un groupe.
   *
   * Aucune URL n'est touchée : pas d'ancre, donc rien que le routeur puisse
   * confondre avec une route. `block: 'start'` avec la marge de défilement
   * posée en CSS arrête le titre sous le bord de la zone et non collé dessus.
   */
  protected allerA(ancre: string): void {
    this.groupeEnVue.set(ancre);
    const cible = this.sections().find(
      (element) => element.nativeElement.dataset['groupe'] === ancre,
    );
    cible?.nativeElement.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  // --- Les changements -----------------------------------------------------
  //
  // Deux temps, et c'est tout le sujet :
  //
  //   `(input)`  → `changer…` : le brouillon seul. Aucun réseau, aucun disque.
  //   `(change)` → `enregistrer()` : une écriture, au relâchement.
  //
  // Un `<select>` et une case à cocher n'ont qu'un temps — `(change)` fait les
  // deux d'un coup, puisqu'il n'y a pas de geste continu à suivre.

  protected changerFond(valeur: string): void {
    this.vue.update((vu) => ({ ...vu, apparence: { ...vu.apparence, fond: valeur as Fond } }));
    void this.enregistrer();
  }

  protected changerVoile(valeur: string): void {
    this.vue.update((vu) => ({ ...vu, apparence: { ...vu.apparence, voile: Number(valeur) } }));
  }

  protected changerMode(valeur: string): void {
    this.vue.update((vu) => ({ ...vu, fenetre: { ...vu.fenetre, mode: valeur as ModeFenetre } }));
    void this.enregistrer();
  }

  protected changerTaille(champ: 'largeur' | 'hauteur', valeur: string): void {
    this.vue.update((vu) => ({ ...vu, fenetre: { ...vu.fenetre, [champ]: Number(valeur) } }));
    void this.enregistrer();
  }

  protected changerJeu(champ: string, valeur: string | number): void {
    this.vue.update((vu) => ({ ...vu, jeu: { ...vu.jeu, [champ]: Number(valeur) } }));
  }

  protected basculerJeu(champ: string, valeur: boolean): void {
    this.vue.update((vu) => ({ ...vu, jeu: { ...vu.jeu, [champ]: valeur } }));
    void this.enregistrer();
  }

  protected changerMemoire(valeur: string): void {
    this.vue.update((vu) => ({
      ...vu,
      lanceur: { ...vu.lanceur, memoireMo: Math.round(Number(valeur) * 1024) },
    }));
  }

  protected changerReduction(valeur: boolean): void {
    this.vue.update((vu) => ({ ...vu, lanceur: { ...vu.lanceur, reduireAuLancement: valeur } }));
    void this.enregistrer();
  }

  /** Écrit le brouillon. Appelée au relâchement, jamais pendant le geste. */
  protected async enregistrer(): Promise<void> {
    await this.incidents.pendant(() => this.reglages.enregistrer(this.vue()));
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
