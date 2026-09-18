import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router, RouterLink, RouterLinkActive } from '@angular/router';

import { Incidents } from '../../noyau/incidents';
import { Session } from '../../noyau/session';

/** Les trois pages, et de quoi les dessiner. */
export interface Onglet {
  readonly chemin: string;
  readonly libelle: string;
  /** Un glyphe, et non une icône : voir le commentaire du gabarit. */
  readonly glyphe: string;
}

/**
 * Le menu de gauche, toujours ouvert.
 *
 * ## Pourquoi il n'est pas repliable
 *
 * Trois entrées. Un bouton pour replier trois entrées coûterait un état à
 * garder, une animation à écrire, et une largeur qui change sous le contenu —
 * pour récupérer cent quatre-vingts pixels sur une fenêtre qui en fait neuf
 * cents.
 *
 * ## La rangée du bas
 *
 * `mt-auto` la colle en bas sans `position: sticky`. Le plan hésitait sur
 * `sticky` à cause du contexte d'empilement ; la question ne se pose pas ici —
 * dans une colonne flex qui occupe toute la hauteur, `mt-auto` suffit et ne
 * crée aucun contexte.
 *
 * Les deux boutons partagent la largeur en deux moitiés égales : `flex-1` sur
 * chacun, dans un conteneur `flex`. C'est ce que Sam a demandé, et c'est aussi
 * ce qui empêche le bouton de déconnexion de grandir avec la longueur de son
 * libellé.
 */
@Component({
  selector: 'app-menu',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink, RouterLinkActive],
  templateUrl: './menu.html',
  styleUrl: './menu.css',
})
export class Menu {
  private readonly session = inject(Session);
  private readonly incidents = inject(Incidents);
  private readonly router = inject(Router);

  protected readonly compte = this.session.compte;
  protected readonly occupe = this.session.occupe;

  protected readonly onglets: readonly Onglet[] = [
    { chemin: '/spawn', libelle: 'Spawn', glyphe: '⌂' },
    { chemin: '/nouvelles', libelle: 'Nouvelles', glyphe: '✉' },
    { chemin: '/configuration', libelle: 'Configuration', glyphe: '⚙' },
  ];

  protected async deconnecter(): Promise<void> {
    await this.incidents.pendant(async () => {
      await this.session.deconnecter();
      // La garde de `/spawn` renverrait de toute façon, mais l'attendre
      // laisserait une page qui parle d'un compte qui n'existe plus le temps
      // d'un rendu. On y va nous-mêmes.
      await this.router.navigate(['/connexion']);
    });
  }
}
