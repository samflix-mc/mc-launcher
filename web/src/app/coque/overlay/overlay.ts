import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { LucideAngularModule } from 'lucide-angular';

import { AlertTriangle, Copy, X } from '../../noyau/icones';

import { Incidents } from '../../noyau/incidents';

/**
 * Ce qui a mal tourné, par-dessus tout le reste.
 *
 * ## Il vit au niveau de `App`, et c'est une contrainte
 *
 * Le menu porte un `backdrop-filter`, qui crée un contexte d'empilement et
 * fait de l'`aside` un bloc conteneur pour ses descendants en
 * `position: fixed`. Un overlay monté DANS le menu serait donc borné au menu,
 * quel que soit son `inset` — il couvrirait cent quatre-vingts pixels de large
 * et laisserait le reste de l'écran cliquable.
 *
 * ## Le flou porte sur l'interface, pas sur le fond
 *
 * Le fond vit sur `body`, hors de tout composant. Flouter `.app` ne le
 * toucherait donc pas — et avec une photographie, l'effet serait de flouter
 * l'interface en laissant l'image nette, ce qui est exactement l'inverse de ce
 * qu'on veut. On floute donc la zone de contenu, et l'overlay reste HORS
 * d'elle : un élément ne peut pas se flouter lui-même sans se flouter.
 */
@Component({
  selector: 'app-overlay',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './overlay.html',
  styleUrl: './overlay.css',
})
export class Overlay {
  private readonly incidents = inject(Incidents);

  protected readonly message = this.incidents.courant;

  protected readonly AlertTriangle = AlertTriangle;
  protected readonly Copy = Copy;
  protected readonly X = X;

  protected fermer(): void {
    this.incidents.fermer();
  }

  protected async copier(): Promise<void> {
    const texte = this.message();
    if (!texte) {
      return;
    }
    // Sans `catch`, un presse-papiers refusé ferait une promesse rejetée dans
    // le vide — et l'erreur d'origine serait remplacée dans la fenêtre par
    // celle de la copie, ce qui est doublement inutile.
    await navigator.clipboard.writeText(texte).catch(() => {});
  }
}
