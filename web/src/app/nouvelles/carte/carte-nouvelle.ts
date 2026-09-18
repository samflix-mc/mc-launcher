import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LucideAngularModule } from 'lucide-angular';

import { ChevronRight } from '../../noyau/icones';
import type { Billet } from '../../noyau/contrats';
import { direLaDate } from '../../noyau/dates';

/** Les trois tailles de tuile du design system. */
export type Variante = 'epinglee' | 'vedette' | 'tuile';

/**
 * Un billet, en TUILE : l'image remplit, un dégradé monte du bas, le texte
 * s'assoit dessus.
 *
 * ## Trois tailles, un seul composant
 *
 * `epinglee` sur Spawn — la tuile haute au liseré doré, une par écran ;
 * `vedette` en tête des Nouvelles, pleine largeur ; `tuile` dans la grille.
 * Le design system les décrit comme des modificateurs de la même chose, et les
 * séparer en trois composants ferait diverger trois fois le même dégradé.
 *
 * ## Elle ouvre la page, pas le navigateur
 *
 * C'est l'écart assumé avec le design system. Chez lui, un billet vit sur le
 * site du serveur et la tuile y renvoie ; ici, le corps est analysé par Rust en
 * arbre typé et rendu DANS la fenêtre — c'est la condition à laquelle le CSP a
 * été desserré, et il n'y a pas d'URL vers quoi partir. Le chevron dit donc
 * « ouvrir ici » et non « sortir », ce qui est la seule chose honnête à
 * promettre.
 */
@Component({
  selector: 'app-carte-nouvelle',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink, LucideAngularModule, NgTemplateOutlet],
  templateUrl: './carte-nouvelle.html',
  styleUrl: './carte-nouvelle.css',
})
export class CarteNouvelle {
  readonly billet = input.required<Billet>();
  readonly variante = input<Variante>('tuile');

  /**
   * La tuile mène-t-elle à la page des nouvelles, ou ouvre-t-elle ici ?
   *
   * Sur Spawn, elle mène : il n'y a rien à lire sur place. Sur la page des
   * nouvelles, elle ouvre le billet dans un dialogue — y mettre un lien vers la
   * page où l'on est déjà ne ferait rien du tout.
   */
  readonly commeLien = input(true);

  /** Émis quand la tuile n'est pas un lien et qu'on la clique. */
  readonly ouvrir = output<Billet>();

  protected readonly ChevronRight = ChevronRight;

  protected readonly quand = computed(() => direLaDate(this.billet().date));

  /** Les classes de la tuile, selon sa taille. */
  protected readonly classes = computed(() => {
    const base = 'hm-glass hm-glass--interactive hm-news';
    switch (this.variante()) {
      case 'epinglee':
        // Le liseré doré : le design system n'en autorise qu'UN par écran, et
        // c'est celui-ci — la seule chose mise en avant sur Spawn.
        return `${base} hm-news--pinned hm-glass--gold`;
      case 'vedette':
        return `${base} hm-news--featured`;
      case 'tuile':
        return base;
    }
  });

  /** L'extrait ne s'affiche que sur les deux grandes tailles. */
  protected readonly montrerExtrait = computed(() => this.variante() !== 'tuile');

  /**
   * Le premier paragraphe, aplati en texte.
   *
   * Le premier PARAGRAPHE et non les premiers caractères du corps : un titre ou
   * une liste en tête donneraient un extrait qui ne ressemble à rien.
   */
  protected readonly extrait = computed(() => {
    const premier = this.billet().corps.find((bloc) => bloc.type === 'paragraphe');
    if (!premier || premier.type !== 'paragraphe') {
      return '';
    }
    return premier.contenu
      .map((morceau) => morceau.texte)
      .join('')
      .trim();
  });
}
