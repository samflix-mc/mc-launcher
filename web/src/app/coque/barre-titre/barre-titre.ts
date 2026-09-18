import { ChangeDetectionStrategy, Component, inject } from '@angular/core';

import { Fenetre } from '../../noyau/fenetre';
import { Marque } from '../../noyau/marque';

/**
 * La barre de titre du launcher, à la place de celle du système.
 *
 * ## Ce qu'on n'a PAS eu à écrire
 *
 * Le redimensionnement par les bords. `tauri-runtime-wry` branche de lui-même
 * un gestionnaire sur la WebView sous Linux, avec une bande de cinq pixels
 * multipliée par le facteur d'échelle. Écrire des zones DOM ferait double
 * emploi et se battrait avec lui.
 *
 * ## La contrainte des huit pixels
 *
 * Aucune cible de clic ne commence à moins de huit pixels du bord. C'est
 * pourquoi le gabarit porte un `pt-bord-fenetre` et que les boutons ont une
 * marge à droite : sans cela, tirer la fenêtre par le haut la REDIMENSIONNE au
 * lieu de la déplacer, parce que le gestionnaire GTK s'exécute avant que
 * WebKit ne dispatche le `mousedown`.
 *
 * Et le piège dans le piège : la garde `!is_maximized()` de
 * `tauri-runtime-wry` fait que la bande redevient cliquable une fois la
 * fenêtre maximisée. Un même bouton se comporte donc différemment selon
 * l'état — et aucun test jsdom ne le verra jamais.
 *
 * ## Pas de `pointer-events: none` sur les enfants
 *
 * Le script de glissement de Tauri accepte `data-tauri-drag-region` en
 * profondeur, et écarte de lui-même `A`, `BUTTON`, `INPUT`, `SELECT`,
 * `TEXTAREA`, `LABEL`, `SUMMARY`, tout `[contenteditable]`, tout `[tabindex]`
 * non négatif et tout `role` interactif. Neutraliser les pointeurs ne servirait
 * à rien et coûterait le survol et le curseur.
 */
@Component({
  selector: 'app-barre-titre',
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './barre-titre.html',
  styleUrl: './barre-titre.css',
})
export class BarreTitre {
  private readonly fenetre = inject(Fenetre);
  protected readonly marque = inject(Marque).vue;
  protected readonly maximisee = this.fenetre.maximisee;

  protected reduire(): void {
    void this.fenetre.reduire();
  }

  protected basculer(): void {
    void this.fenetre.basculerMaximisee();
  }

  protected fermer(): void {
    void this.fenetre.fermer();
  }
}
