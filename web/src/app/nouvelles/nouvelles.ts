import { ChangeDetectionStrategy, Component, inject } from '@angular/core';

import { Incidents } from '../noyau/incidents';
import { Nouvelles } from '../noyau/nouvelles';
import { CorpsBillet } from './corps/corps';
import { direLaDate } from './dates';

/**
 * Le fil de nouvelles, en entier.
 *
 * Le nom de classe est `PageNouvelles` et non `Nouvelles` : le service
 * s'appelle déjà ainsi, et deux symboles du même nom dans le même fichier
 * obligeraient à en renommer un à l'import — ce qui se lit mal.
 */
@Component({
  selector: 'app-nouvelles',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [CorpsBillet],
  templateUrl: './nouvelles.html',
  styleUrl: './nouvelles.css',
})
export class PageNouvelles {
  private readonly nouvelles = inject(Nouvelles);
  private readonly incidents = inject(Incidents);

  protected readonly fil = this.nouvelles.fil;
  protected readonly chargement = this.nouvelles.chargement;

  constructor() {
    void this.incidents.pendant(() => this.nouvelles.charger());
  }

  protected async recharger(): Promise<void> {
    await this.incidents.pendant(() => this.nouvelles.recharger());
  }

  protected quand(iso: string): string {
    return direLaDate(iso);
  }
}
