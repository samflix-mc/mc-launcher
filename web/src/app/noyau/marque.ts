import { Injectable, inject, signal } from '@angular/core';

import type { Marque as MarqueVue } from './contrats';
import { Pont } from './pont';

/**
 * Sous quel nom le launcher se présente.
 *
 * ## Pourquoi un service à lui seul
 *
 * Le nom n'appartient ni à la session, ni au pack, ni aux incidents — et DEUX
 * consommateurs le lisent : la barre de titre, qui vit dans la coque, et le
 * bandeau de Spawn. La barre de titre ne peut pas le demander à Spawn : elle
 * est au-dessus de lui dans l'arbre, et elle existe même quand Spawn n'est pas
 * la page affichée.
 *
 * Il est demandé UNE fois, au premier accès, et gardé : il est figé à la
 * compilation côté Rust, donc il ne changera pas de la session.
 */
@Injectable({ providedIn: 'root' })
export class Marque {
  private readonly pont = inject(Pont);

  /**
   * Un défaut qui n'est pas « chargement… ».
   *
   * La barre de titre est visible dès la première image : y afficher un texte
   * d'attente ferait clignoter le nom du launcher à chaque ouverture. Le
   * défaut est donc le nom probable, remplacé sans qu'on le remarque.
   */
  readonly vue = signal<MarqueVue>({ nom: 'samflix-mc', sceau: 'SA' });

  private demandee = false;

  /** Demande la marque à Rust, une fois pour la session. */
  async charger(): Promise<void> {
    if (this.demandee || !this.pont.disponible) {
      return;
    }
    this.demandee = true;
    try {
      this.vue.set(await this.pont.marque());
    } catch {
      // Le défaut reste affiché. Une marque qu'on n'a pas pu lire n'empêche
      // pas de jouer, et un bandeau d'erreur pour un nom de fenêtre serait
      // hors de proportion.
      this.demandee = false;
    }
  }
}
