import { Injectable, computed, signal } from '@angular/core';

/** Le ton d'une notification, tel que le design system les nomme. */
export type Ton = 'success' | 'danger' | 'info' | 'warning' | 'progress';

/** Ce qui vient de se passer. */
export interface Avis {
  readonly id: number;
  readonly ton: Ton;
  /** Ce qui s'est passé, en cinq mots. */
  readonly titre: string;
  /** Le nombre, ou la marche à suivre. Une ligne. */
  readonly texte: string | null;
  /** L'instant, en RFC 3339 — le même format que les billets. */
  readonly quand: string;
  readonly lu: boolean;
}

/**
 * Combien d'avis le centre garde.
 *
 * Cinquante, comme le design system le pose. Au-delà, un launcher laissé
 * ouvert une semaine accumulerait un journal que personne ne déroule, et dont
 * le seul effet serait d'allonger le rendu du panneau.
 */
const MEMOIRE = 50;

/** Combien de toasts se voient à la fois. Les autres attendent leur tour. */
const TOASTS_VISIBLES = 3;

/**
 * Combien de temps un toast reste, par ton, en millisecondes.
 *
 * Les durées viennent du design system : six secondes pour ce qui informe,
 * douze pour ce qui demande une décision. `progress` n'y est pas — un toast de
 * progression reste jusqu'à ce que le travail finisse, et c'est l'appelant qui
 * le retire en le remplaçant.
 */
const DUREES: Record<Ton, number | null> = {
  success: 6000,
  info: 6000,
  danger: 12_000,
  warning: 12_000,
  progress: null,
};

/**
 * Les notifications du launcher, sur les deux niveaux que la fenêtre porte.
 *
 * ## Les trois niveaux du design system, et celui qui manque
 *
 * Le design system en décrit trois : le toast, transitoire, en bas à gauche ;
 * le centre, persistant, sous la cloche ; et la notification du SYSTÈME, pour
 * les mêmes événements quand le launcher est caché derrière le jeu.
 *
 * Les deux premiers sont ici. Le troisième ne peut pas l'être depuis le front :
 * il demande `@tauri-apps/plugin-notification`, donc une dépendance Rust, une
 * permission dans `capabilities/default.json` et une clé de réglages
 * « Notifications : launcher et système / launcher seul / aucune ». C'est un
 * travail de backend, et l'annoncer ici plutôt que de le taire évite qu'on
 * croie le niveau perdu.
 *
 * ## Tout toast est AUSSI écrit au centre
 *
 * C'est la règle du design system, et elle a une raison : un toast dure six
 * secondes, et six secondes suffisent à regarder ailleurs. Rien de ce qui a été
 * dit ne doit disparaître sans laisser de trace.
 *
 * L'inverse n'est pas vrai : on peut écrire au centre SANS toast — c'est ce que
 * font les incidents, que la fenêtre montre déjà dans son dialogue, et qu'un
 * toast répéterait par-dessus.
 */
@Injectable({ providedIn: 'root' })
export class Notifications {
  /** Le journal, du plus récent au plus ancien. */
  readonly journal = signal<readonly Avis[]>([]);

  /** Les identifiants des avis actuellement affichés en toast. */
  private readonly visibles = signal<readonly number[]>([]);

  /** Le panneau sous la cloche est-il ouvert ? */
  readonly panneauOuvert = signal(false);

  readonly nonLus = computed(() => this.journal().filter((avis) => !avis.lu).length);

  /**
   * Les toasts, du plus ancien au plus récent.
   *
   * Le design system veut le plus récent EN BAS : la pile monte, et l'œil
   * revient toujours au même endroit. `.hm-toasts` aligne son contenu en bas,
   * l'ordre du tableau suffit donc.
   */
  readonly toasts = computed(() => {
    const vus = this.visibles();
    return this.journal()
      .filter((avis) => vus.includes(avis.id))
      .slice(0, TOASTS_VISIBLES)
      .reverse();
  });

  private suivant = 1;

  /**
   * Un événement qui mérite un toast.
   *
   * Rend l'identifiant, ce qui permet à l'appelant de le retirer lui-même — un
   * toast de progression, qu'on remplace quand le travail finit.
   */
  signaler(ton: Ton, titre: string, texte: string | null = null): number {
    const id = this.inscrire(ton, titre, texte);
    this.visibles.update((vus) => [id, ...vus]);

    const duree = DUREES[ton];
    if (duree !== null) {
      setTimeout(() => this.fermerToast(id), duree);
    }
    return id;
  }

  /**
   * Un événement que la fenêtre montre DÉJÀ autrement.
   *
   * Il entre au centre et n'ouvre aucun toast. C'est le cas des incidents : le
   * dialogue les affiche en grand, et un toast par-dessus dirait deux fois la
   * même chose au même instant.
   */
  archiver(ton: Ton, titre: string, texte: string | null = null): number {
    return this.inscrire(ton, titre, texte);
  }

  /** Retire un toast de l'écran. L'avis reste au centre. */
  fermerToast(id: number): void {
    this.visibles.update((vus) => vus.filter((vu) => vu !== id));
  }

  basculerPanneau(): void {
    const ouvert = !this.panneauOuvert();
    this.panneauOuvert.set(ouvert);
    // Ouvrir le panneau, c'est avoir vu ce qu'il porte. Marquer à la fermeture
    // laisserait la pastille allumée pendant qu'on lit dessous.
    if (ouvert) {
      this.toutLire();
    }
  }

  fermerPanneau(): void {
    this.panneauOuvert.set(false);
  }

  toutLire(): void {
    this.journal.update((avis) => avis.map((un) => (un.lu ? un : { ...un, lu: true })));
  }

  vider(): void {
    this.journal.set([]);
    this.visibles.set([]);
  }

  private inscrire(ton: Ton, titre: string, texte: string | null): number {
    const id = this.suivant++;
    const avis: Avis = {
      id,
      ton,
      titre,
      texte,
      quand: new Date().toISOString(),
      lu: false,
    };
    this.journal.update((journal) => [avis, ...journal].slice(0, MEMOIRE));
    return id;
  }
}
