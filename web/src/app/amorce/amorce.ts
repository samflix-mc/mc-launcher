import { ChangeDetectionStrategy, Component, inject, input } from '@angular/core';

import { Marque } from '../noyau/marque';
import { unePhrase } from './phrases';

/**
 * L'écran de démarrage : logo, chargeur, phrase.
 *
 * ## Ce qu'il ne doit PAS couvrir
 *
 * La barre de titre. `statut()` enchaîne deux allers-retours réseau, et sur un
 * réseau lent ou derrière un portail captif, cela dure. Une amorce plein écran
 * laisserait alors une fenêtre sans bouton système — puisqu'on les a
 * retirés — et sans bouton applicatif — puisqu'ils seraient cachés dessous.
 * Le joueur n'aurait aucun moyen de fermer.
 *
 * La coque monte donc la barre de titre AU-DESSUS de l'amorce, et l'amorce ne
 * couvre que la zone de contenu.
 *
 * ## La phrase est tirée UNE fois
 *
 * Dans un champ, pas dans un getter. Un getter la retirerait à chaque
 * détection de changement, et elle changerait plusieurs fois par seconde — le
 * contraire de ce qu'on veut d'un texte à lire.
 */
@Component({
  selector: 'app-amorce',
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './amorce.html',
  styleUrl: './amorce.css',
})
export class Amorce {
  /**
   * Ce que l'amorce annonce, si l'appelant a quelque chose de précis à dire.
   *
   * À défaut, une phrase tirée au sort. Le paramètre existe pour le jour où
   * l'on préférera des états réels — « vérification de la session… » — sans
   * avoir à rouvrir le composant.
   */
  readonly phrase = input<string | null>(null);

  protected readonly marque = inject(Marque).vue;

  /** Tirée une fois, à la construction. */
  protected readonly parDefaut = unePhrase();
}
