import { Injectable, inject, signal } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { Pont } from './pont';

/**
 * Les boutons de la barre de titre.
 *
 * ## Pourquoi ce service existe
 *
 * `decorations: false` retire la barre du système : réduire, agrandir et
 * fermer n'ont plus de bouton, et c'est à nous de les redonner. Les
 * permissions correspondantes ne sont PAS dans `core:window:default` — elles
 * sont déclarées nommément dans `capabilities/default.json`, et leur absence
 * se manifeste par un refus d'ACL qu'on ne voit qu'en build packagé.
 *
 * ## Ce qu'on n'a PAS eu à écrire
 *
 * Le redimensionnement par les bords. `tauri-runtime-wry` branche de lui-même
 * un gestionnaire sur la WebView sous Linux, avec une bande de cinq pixels
 * multipliée par le facteur d'échelle. Ajouter des zones DOM ferait double
 * emploi, et se battrait avec lui.
 *
 * La contrepartie est une CONTRAINTE DE GABARIT, pas du code : aucune cible de
 * clic ne commence à moins de huit pixels d'un bord. Sans elle, tirer la
 * fenêtre par le haut la redimensionne au lieu de la déplacer — le
 * gestionnaire GTK s'exécute avant que WebKit ne dispatche le `mousedown`.
 */
@Injectable({ providedIn: 'root' })
export class Fenetre {
  private readonly pont = inject(Pont);

  /** La fenêtre est-elle maximisée ? Pour changer l'icône du bouton. */
  readonly maximisee = signal(false);

  async reduire(): Promise<void> {
    if (!this.pont.disponible) {
      return;
    }
    await getCurrentWindow().minimize();
  }

  async basculerMaximisee(): Promise<void> {
    if (!this.pont.disponible) {
      return;
    }
    const fenetre = getCurrentWindow();
    await fenetre.toggleMaximize();
    this.maximisee.set(await fenetre.isMaximized());
  }

  async fermer(): Promise<void> {
    if (!this.pont.disponible) {
      return;
    }
    await getCurrentWindow().close();
  }

  /** Relit l'état au démarrage : la fenêtre peut s'ouvrir déjà maximisée. */
  async observer(): Promise<void> {
    if (!this.pont.disponible) {
      return;
    }
    this.maximisee.set(await getCurrentWindow().isMaximized());
  }
}
