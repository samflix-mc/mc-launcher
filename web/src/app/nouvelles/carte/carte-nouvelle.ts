import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { RouterLink } from '@angular/router';

import type { Billet } from '../../noyau/contrats';
import { direLaDate } from '../dates';

/**
 * La dernière nouvelle, en carte, sur l'écran principal.
 *
 * ## Elle ne rend PAS le corps
 *
 * Un extrait, et un lien vers la page. Rendre l'arbre complet ferait qu'un
 * billet de trois écrans repousserait le bouton JOUER hors de la fenêtre —
 * c'est-à-dire que la carte des news mangerait l'écran qu'elle décore.
 *
 * L'extrait est tiré du PREMIER PARAGRAPHE et non des premiers caractères du
 * markdown : un titre ou une liste en tête donneraient un extrait qui ne
 * ressemble à rien.
 */
@Component({
  selector: 'app-carte-nouvelle',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink],
  templateUrl: './carte-nouvelle.html',
  styleUrl: './carte-nouvelle.css',
})
export class CarteNouvelle {
  readonly billet = input.required<Billet>();

  protected readonly quand = computed(() => direLaDate(this.billet().date));

  /** Le premier paragraphe, aplati en texte. */
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
