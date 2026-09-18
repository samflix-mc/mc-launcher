import { ChangeDetectionStrategy, Component, inject, input } from '@angular/core';

import { Marque } from '../noyau/marque';
import { unePhrase } from './phrases';

/**
 * L'écran de démarrage, dans la fenêtre — le même dessin que celui de Tauri.
 *
 * ## Les trois écrans sont identiques, et c'est tout le point
 *
 * Il y en a trois à la suite : la fenêtre d'écran de démarrage de Tauri
 * (`public/splash.html`), l'amorce statique d'`index.html`, et celui-ci. Les
 * trois portent `.hm-splash`, `.hm-wordmark` et la même progression : aucun
 * passage ne se voit, et il n'y a qu'un seul dessin à tenir à jour.
 *
 * ## Ce qu'il ne doit PAS couvrir
 *
 * La barre de titre. `statut()` enchaîne deux allers-retours réseau, et sur un
 * réseau lent ou derrière un portail captif, cela dure. Une amorce plein écran
 * laisserait alors une fenêtre sans bouton système — puisqu'on les a
 * retirés — et sans bouton applicatif — puisqu'ils seraient cachés dessous.
 *
 * ## La phrase est tirée UNE fois
 *
 * Dans un champ, pas dans un accesseur : un accesseur la retirerait à chaque
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
   * À défaut, une phrase tirée au sort.
   */
  readonly phrase = input<string | null>(null);

  protected readonly marque = inject(Marque).vue;

  /** Tirée une fois, à la construction. */
  protected readonly parDefaut = unePhrase();
}
