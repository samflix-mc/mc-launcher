import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import { Compass, Newspaper, SlidersHorizontal } from '../../noyau/icones';

/** Une section, et de quoi la dessiner. */
export interface Onglet {
  readonly chemin: string;
  readonly libelle: string;
  readonly icone: LucideIconData;
}

/**
 * La pilule de navigation, flottante et centrée en haut de la page.
 *
 * ## Elle remplace le menu de gauche, et ce n'est pas un détail de goût
 *
 * Le menu latéral prenait cent quatre-vingts pixels sur toute la hauteur pour
 * trois entrées — c'est-à-dire qu'il mangeait une bande de l'image sur toute la
 * fenêtre, en permanence. Le design system met les trois sections dans une
 * pilule en verre qui ne coûte qu'une ligne en haut : le milieu de l'écran
 * reste à l'image, ce qui est tout le sujet de cette direction artistique.
 *
 * ## Trois entrées, dans cet ordre, toujours visibles
 *
 * Spawn est la page d'arrivée et la navigation ne se souvient de rien : rouvrir
 * le launcher rouvre Spawn. Une quatrième section se demanderait d'abord si
 * elle n'appartient pas à la Configuration.
 *
 * ## Ce qui n'est plus ici
 *
 * « Se déconnecter » descendait dans cette barre ; il est maintenant dans le
 * menu du badge joueur, qui est l'endroit où l'on va quand on pense à son
 * compte. Et il n'apparaît plus du tout tant qu'il n'y a pas de session : la
 * pilule elle-même n'existe pas avant la connexion.
 */
@Component({
  selector: 'app-nav',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule, RouterLink, RouterLinkActive],
  templateUrl: './nav.html',
  styleUrl: './nav.css',
})
export class Nav {
  protected readonly onglets: readonly Onglet[] = [
    { chemin: '/spawn', libelle: 'Spawn', icone: Compass },
    { chemin: '/nouvelles', libelle: 'Nouvelles', icone: Newspaper },
    { chemin: '/configuration', libelle: 'Configuration', icone: SlidersHorizontal },
  ];
}
