import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { LucideAngularModule } from 'lucide-angular';

import { Copy, TriangleAlert } from '../../noyau/icones';
import { Incidents } from '../../noyau/incidents';

/**
 * Ce qui a mal tourné, par-dessus tout le reste.
 *
 * ## La forme vient du design system, et elle a une règle
 *
 * Ce qui s'est passé, puis la marche à suivre, en deux phrases courtes. Le
 * détail technique — le message que Rust a rendu — est ce qu'on demande de
 * recopier : il est sélectionnable, dans la face mono, et il porte le bouton
 * qui le copie.
 *
 * ## Il vit au niveau de la FENÊTRE, et c'est une contrainte
 *
 * La pilule de navigation et les panneaux portent un `backdrop-filter`, qui
 * crée un contexte d'empilement et fait d'eux un bloc conteneur pour leurs
 * descendants positionnés. Un dialogue monté dedans serait borné à leur
 * largeur, et laisserait le reste de l'écran cliquable.
 *
 * ## Pas de flou sur le contenu
 *
 * L'ancienne version floutait la zone de contenu. Le design system voile
 * plutôt : `.hm-scrim` assombrit, ce qui coûte un composite au lieu d'un flou —
 * et empiler deux flous coûte cher sur une WebView sans DMA-BUF, c'est-à-dire
 * exactement la configuration NVIDIA que ce launcher rencontre.
 */
@Component({
  selector: 'app-incident',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './incident.html',
  styleUrl: './incident.css',
})
export class Incident {
  private readonly incidents = inject(Incidents);

  protected readonly message = this.incidents.courant;

  protected readonly TriangleAlert = TriangleAlert;
  protected readonly Copy = Copy;

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
