import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { LucideAngularModule } from 'lucide-angular';

import { Globe, RefreshCw, X } from '../noyau/icones';
import type { Billet } from '../noyau/contrats';
import { direLaDate } from '../noyau/dates';
import { Incidents } from '../noyau/incidents';
import { Nouvelles } from '../noyau/nouvelles';
import { CarteNouvelle } from './carte/carte-nouvelle';
import { CorpsBillet } from './corps/corps';

/**
 * Le fil de nouvelles : une tuile vedette, puis une grille.
 *
 * Le nom de classe est `PageNouvelles` et non `Nouvelles` : le service s'appelle
 * déjà ainsi, et deux symboles du même nom dans le même fichier obligeraient à
 * en renommer un à l'import — ce qui se lit mal.
 *
 * ## L'écart assumé avec le design system
 *
 * Chez lui, une tuile ouvre le billet sur le SITE du serveur, dans le
 * navigateur, et le launcher n'affiche jamais de corps. Ici, le corps est
 * analysé par Rust en arbre typé et rendu dans la fenêtre — c'est la condition
 * à laquelle le CSP a été desserré, et c'est ce qui permet de n'avoir aucun
 * `innerHTML` dans ce dépôt.
 *
 * La tuile ouvre donc un dialogue de lecture. Ce qui est repris du design
 * system, c'est la forme : la vedette, la grille, le dégradé, le chevron. Ce
 * qui ne l'est pas, c'est la destination — et il valait mieux la changer que de
 * mentir sur ce que la tuile fait.
 */
@Component({
  selector: 'app-nouvelles',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [CarteNouvelle, CorpsBillet, LucideAngularModule],
  templateUrl: './nouvelles.html',
  styleUrl: './nouvelles.css',
})
export class PageNouvelles {
  private readonly nouvelles = inject(Nouvelles);
  private readonly incidents = inject(Incidents);

  protected readonly fil = this.nouvelles.fil;
  protected readonly chargement = this.nouvelles.chargement;
  protected readonly vedette = this.nouvelles.epinglee;

  /** Le billet ouvert en lecture, ou `null`. */
  protected readonly lecture = signal<Billet | null>(null);

  protected readonly RefreshCw = RefreshCw;
  protected readonly Globe = Globe;
  protected readonly X = X;

  /**
   * La grille : tout le fil, la vedette comprise.
   *
   * Le design system le dit : « la vedette apparaît aussi dans la grille dans
   * l'ordre des dates, pour que la liste reste complète ». Retirer le billet mis
   * en avant ferait un trou dans la chronologie, et l'on chercherait longtemps
   * pourquoi il manque.
   */
  protected readonly grille = computed(() => this.fil()?.billets ?? []);

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
