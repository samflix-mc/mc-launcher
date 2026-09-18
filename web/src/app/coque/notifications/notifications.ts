import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import { CircleCheck, CircleX, Download, Info, Trash2, TriangleAlert, X } from '../../noyau/icones';
import { direLaDate } from '../../noyau/dates';
import { Notifications as Service, type Ton } from '../../noyau/notifications';

/**
 * Les deux niveaux visibles du design system : les toasts, et le centre.
 *
 * ## Un seul composant pour les deux, et c'est délibéré
 *
 * Ils portent le même vocabulaire — le même ton, la même icône, le même titre —
 * et tout toast est aussi écrit au centre. Les séparer en deux composants
 * obligerait à recopier la correspondance ton → icône, qui divergerait au
 * premier ajout : un toast doré et une ligne grise pour le même événement.
 *
 * ## Ils vivent au niveau de la FENÊTRE
 *
 * `.hm-toasts` est en bas à gauche, `.hm-notif` sous la cloche : les deux sont
 * positionnés par rapport à la fenêtre. Monter ce composant dans la page le
 * ferait border par le premier ancêtre qui porte un `backdrop-filter` — la
 * pilule de navigation, par exemple, qui en crée un.
 */
@Component({
  selector: 'app-notifications',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './notifications.html',
  styleUrl: './notifications.css',
})
export class Notifications {
  private readonly service = inject(Service);

  protected readonly toasts = this.service.toasts;
  protected readonly journal = this.service.journal;
  protected readonly panneauOuvert = this.service.panneauOuvert;

  protected readonly X = X;
  protected readonly Trash2 = Trash2;

  /**
   * L'icône d'un ton.
   *
   * La couleur seule ne suffirait pas : le design system l'interdit
   * explicitement — « success et danger ne se distinguent que par la teinte » —
   * et une icône est ce qui fait la différence pour qui la distingue mal.
   */
  protected icone(ton: Ton): LucideIconData {
    switch (ton) {
      case 'success':
        return CircleCheck;
      case 'danger':
        return CircleX;
      case 'warning':
        return TriangleAlert;
      case 'progress':
        return Download;
      case 'info':
        return Info;
    }
  }

  protected quand(iso: string): string {
    return direLaDate(iso);
  }

  protected fermer(id: number): void {
    this.service.fermerToast(id);
  }

  protected fermerPanneau(): void {
    this.service.fermerPanneau();
  }

  protected vider(): void {
    this.service.vider();
  }
}
