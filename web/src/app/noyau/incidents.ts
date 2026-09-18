import { Injectable, computed, inject, signal } from '@angular/core';

import { Notifications } from './notifications';
import { messageDErreur } from './pont';

/**
 * Ce qui a mal tourné, et que le joueur doit voir.
 *
 * ## Un seul endroit, et un seul à la fois
 *
 * Une erreur floute tout le reste et bloque les clics : impossible de la
 * manquer, là où un bandeau en bas de page passait inaperçu dès qu'on avait
 * fait défiler.
 *
 * Une seule à la fois, et c'est la dernière qui gagne : empiler les erreurs
 * demanderait au joueur de les fermer une par une, alors que la première est
 * presque toujours la cause des suivantes.
 *
 * ## Pourquoi l'overlay n'est pas dans le menu
 *
 * Le menu porte un `backdrop-filter`, qui crée un contexte d'empilement ET
 * fait de l'`aside` un bloc conteneur pour ses descendants en
 * `position: fixed`. Un overlay qui vivrait dedans serait donc borné au menu,
 * quel que soit son `inset`. Il est monté au niveau de `App`, et c'est la
 * contrainte à tenir.
 */
@Injectable({ providedIn: 'root' })
export class Incidents {
  /** Le message affiché, ou `null`. */
  private readonly notifications = inject(Notifications);

  readonly courant = signal<string | null>(null);

  readonly ouvert = computed(() => this.courant() !== null);

  /** Signale ce qui vient d'échouer. Traduit ce que Rust a envoyé. */
  signaler(cause: unknown): void {
    const message = messageDErreur(cause);
    // Le journal du navigateur garde la cause brute : le message affiché est
    // mis en forme pour un joueur, et l'on veut les deux.
    console.error('[incident]', cause);
    this.courant.set(message);
    // Au centre, mais SANS toast : le dialogue montre déjà l'incident en grand,
    // et un toast par-dessus dirait deux fois la même chose au même instant.
    // Ce qu'on gagne est la trace : le dialogue se ferme, le centre garde.
    this.notifications.archiver('danger', "Quelque chose n'a pas fonctionné", message);
  }

  fermer(): void {
    this.courant.set(null);
  }

  /**
   * Enveloppe un appel : signale l'erreur, et rend `null` au lieu de rejeter.
   *
   * Chaque appel faisait la même séquence, avec chaque fois une occasion
   * d'oublier le `catch` — et une promesse rejetée dans le vide, avec une
   * erreur que personne ne voit.
   */
  async pendant<T>(action: () => Promise<T>): Promise<T | null> {
    this.fermer();
    try {
      return await action();
    } catch (cause) {
      this.signaler(cause);
      return null;
    }
  }
}
