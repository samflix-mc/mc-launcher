import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, inject, input } from '@angular/core';

import type { Bloc, Inline } from '../../noyau/contrats';
import { Incidents } from '../../noyau/incidents';
import { Pont } from '../../noyau/pont';

/**
 * Le corps d'un billet, rendu depuis l'arbre typé.
 *
 * ## Zéro `innerHTML`, et c'est la raison d'être de ce composant
 *
 * Le CSP du launcher desserre `style-src` jusqu'à `'unsafe-inline'`. Ce
 * desserrage ne tient qu'à une condition : aucun balisage ne vient d'ailleurs
 * que du compilateur Angular.
 *
 * Le markdown est donc analysé EN RUST vers des variantes fermées, et ce
 * composant les parcourt avec des `@switch`. Il ne reçoit jamais une chaîne
 * qu'il devrait interpréter — seulement du texte à afficher et des structures
 * à parcourir. `pnpm invariants` vérifie qu'aucun `innerHTML` ne traîne ; ce
 * composant est ce qui rend l'invariant tenable.
 *
 * ## Les liens sortent par le système
 *
 * Aucun `href` n'est posé sur un `<a>` : un clic irait dans la fenêtre, que le
 * greffon de navigation de Rust refuserait — en laissant le joueur devant un
 * lien qui ne fait rien. Ils passent par `ouvrirPage()`, qui les confie au
 * navigateur du système.
 */
@Component({
  selector: 'app-corps-billet',
  changeDetection: ChangeDetectionStrategy.OnPush,
  // Le seul import du composant : le gabarit des fragments est écrit une fois
  // et réutilisé par les trois blocs qui en contiennent. Le recopier donnerait
  // trois occasions d'oublier un cas — et un cas oublié affiche du vide, pas
  // une erreur.
  imports: [NgTemplateOutlet],
  templateUrl: './corps.html',
  styleUrl: './corps.css',
})
export class CorpsBillet {
  readonly blocs = input.required<readonly Bloc[]>();

  private readonly pont = inject(Pont);
  private readonly incidents = inject(Incidents);

  protected async ouvrir(inline: Inline): Promise<void> {
    if (inline.type !== 'lien') {
      return;
    }
    await this.incidents.pendant(() => this.pont.ouvrirPage(inline.href));
  }
}
