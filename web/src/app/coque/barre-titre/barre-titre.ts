import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { LucideAngularModule } from 'lucide-angular';

import { Bell, Copy, Minus, Square, X } from '../../noyau/icones';
import { Fenetre } from '../../noyau/fenetre';
import { Marque } from '../../noyau/marque';
import { Notifications } from '../../noyau/notifications';
import { Pack } from '../../noyau/pack';

/**
 * La barre de titre du launcher, à la place de celle du système.
 *
 * ## Ce qu'elle porte, dans l'ordre du design system
 *
 * Le nom du launcher en face pixel, un séparateur, le nom du pack en cours ;
 * puis, poussés à droite, la cloche des notifications et les trois contrôles de
 * fenêtre. Les contrôles font quarante-six pixels de large parce que c'est la
 * largeur des boutons de légende de Windows — un launcher qui la change se
 * remarque, et jamais en bien.
 *
 * ## Ce qu'on n'a PAS eu à écrire
 *
 * Le redimensionnement par les bords. `tauri-runtime-wry` branche de lui-même
 * un gestionnaire sur la WebView sous Linux, avec une bande de cinq pixels
 * multipliée par le facteur d'échelle. Ce qui manquait n'était pas le geste
 * mais le CURSEUR, et il est posé par `.hm-bords`, dans la coque.
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
  imports: [LucideAngularModule],
  templateUrl: './barre-titre.html',
  styleUrl: './barre-titre.css',
})
export class BarreTitre {
  private readonly fenetre = inject(Fenetre);
  private readonly notifications = inject(Notifications);

  protected readonly marque = inject(Marque).vue;
  protected readonly maximisee = this.fenetre.maximisee;
  protected readonly nonLus = this.notifications.nonLus;
  protected readonly panneauOuvert = this.notifications.panneauOuvert;

  private readonly etat = inject(Pack).etat;

  /**
   * Ce qui suit le séparateur : le nom du pack.
   *
   * Le design system y met le nom du serveur. Nous n'en avons qu'un, et ce que
   * le joueur reconnaît est le nom du modpack — c'est ce que le verrou publie,
   * et c'est ce qui change quand le serveur change de saison.
   */
  protected readonly pack = computed(() => this.etat()?.nom ?? null);

  // Les nœuds d'icône sont passés au gabarit comme des valeurs : une faute de
  // frappe est alors une erreur TypeScript, là où un registre résolu par nom
  // rendrait un vide sans rien dire.
  protected readonly Minus = Minus;
  protected readonly Square = Square;
  protected readonly Copy = Copy;
  protected readonly X = X;
  protected readonly Bell = Bell;

  protected basculerNotifications(): void {
    this.notifications.basculerPanneau();
  }

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
