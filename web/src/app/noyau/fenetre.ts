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

  /**
   * L'étiquette de la fenêtre où ce front tourne, ou `null` hors de Tauri.
   *
   * Le launcher en ouvre deux qui chargent la MÊME application Angular :
   * « main » et « connexion ». Elles n'ont pas le même travail — l'une ouvre la
   * fenêtre de connexion quand la session manque, l'autre EST cette fenêtre —
   * et rien dans l'URL ne les distingue, puisqu'elles partagent la route.
   *
   * Lu une fois, au démarrage : une étiquette ne change pas.
   */
  readonly etiquette: string | null = this.pont.dansLaFenetre ? getCurrentWindow().label : null;

  /** Sommes-nous dans la fenêtre principale ? Faux dans celle de connexion. */
  readonly estPrincipale = this.etiquette === 'main';

  /**
   * Sommes-nous dans une fenêtre que le launcher ferme lui-même ?
   *
   * La fenêtre de connexion en est une : quand la session s'ouvre, Rust la
   * referme et montre la principale. Naviguer ailleurs dedans reviendrait à
   * dessiner Spawn dans une fenêtre de quatre cent quarante pixels, le temps
   * qu'elle disparaisse.
   */
  readonly dansUneFenetreDediee = this.etiquette !== null && this.etiquette !== 'main';

  async reduire(): Promise<void> {
    if (!this.pont.dansLaFenetre) {
      return;
    }
    await getCurrentWindow().minimize();
  }

  async basculerMaximisee(): Promise<void> {
    if (!this.pont.dansLaFenetre) {
      return;
    }
    const fenetre = getCurrentWindow();
    await fenetre.toggleMaximize();
    this.maximisee.set(await fenetre.isMaximized());
  }

  async fermer(): Promise<void> {
    if (!this.pont.dansLaFenetre) {
      return;
    }
    await getCurrentWindow().close();
  }

  /** Relit l'état au démarrage : la fenêtre peut s'ouvrir déjà maximisée. */
  async observer(): Promise<void> {
    if (!this.pont.dansLaFenetre) {
      return;
    }
    this.maximisee.set(await getCurrentWindow().isMaximized());
  }
}
