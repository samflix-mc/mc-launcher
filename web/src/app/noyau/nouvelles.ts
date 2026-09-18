import { Injectable, computed, inject, signal } from '@angular/core';

import type { Fil } from './contrats';
import { Pont } from './pont';

/**
 * Le fil de nouvelles du réseau.
 *
 * ## Ce que ce service ne fait PAS
 *
 * Il ne trie rien, ne filtre rien, n'analyse rien. Tout cela est en Rust —
 * pour que la mutation l'atteigne, et parce que le markdown y devient un arbre
 * typé plutôt que du HTML. Ce service garde un résultat et dit s'il est en
 * train d'arriver.
 *
 * ## Le fil est gardé pour la session
 *
 * Revenir sur la page ne le redemande pas. Un fil de nouvelles change
 * plusieurs fois par mois, pas plusieurs fois par minute, et le recharger à
 * chaque navigation ferait clignoter la page pour rien. `recharger()` existe
 * pour le geste explicite.
 */
@Injectable({ providedIn: 'root' })
export class Nouvelles {
  private readonly pont = inject(Pont);

  readonly fil = signal<Fil | null>(null);
  readonly chargement = signal(false);

  /** Le billet le plus récent, pour la carte de Spawn. */
  readonly derniere = computed(() => this.fil()?.billets[0] ?? null);

  /** Charge le fil, une fois pour la session. */
  async charger(): Promise<void> {
    if (this.fil() || this.chargement() || !this.pont.disponible) {
      return;
    }
    await this.recharger();
  }

  /**
   * Redemande le fil, même s'il est déjà là.
   *
   * L'erreur REMONTE plutôt que d'être avalée : c'est à l'appelant de décider
   * si elle mérite un overlay — sur la page des nouvelles, oui ; sur la carte
   * de Spawn, non, où elle se contente de laisser la carte vide.
   */
  async recharger(): Promise<void> {
    this.chargement.set(true);
    try {
      this.fil.set(await this.pont.nouvelles());
    } finally {
      this.chargement.set(false);
    }
  }
}
