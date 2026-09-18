import { Injectable, inject, signal } from '@angular/core';

import type { Ecran, Reglages as ReglagesVue } from './contrats';
import { Pont } from './pont';

/**
 * Les défauts du front, identiques à ceux de Rust.
 *
 * Ils servent avant la première réponse, et hors de la fenêtre Tauri. Les
 * garder alignés est une servitude ; l'alternative — n'afficher aucun réglage
 * tant que Rust n'a pas répondu — ferait clignoter toute la page de
 * configuration à chaque ouverture.
 */
const DEFAUTS: ReglagesVue = {
  schema: 1,
  jeu: { renderDistance: 12, simulationDistance: 10, maxFps: 120, guiScale: 0, vsync: true },
  fenetre: { mode: 'fenetree', largeur: 1280, hauteur: 720 },
  lanceur: { memoireMo: 4096, reduireAuLancement: true },
  apparence: { fond: 'spawn', voile: 0.55 },
};

/**
 * Ce que le joueur a réglé.
 *
 * ## L'apparence est appliquée ICI, sur `<html>`
 *
 * Le fond et le voile sont posés en propriétés personnalisées sur l'élément
 * racine, depuis ce service, et non par un style de composant. La raison est
 * mécanique : l'encapsulation émulée d'Angular réécrit les sélecteurs, et un
 * `:root { … }` écrit dans un composant deviendrait
 * `:root[_ngcontent-abc] { … }` — qui ne désigne rien.
 */
@Injectable({ providedIn: 'root' })
export class Reglages {
  private readonly pont = inject(Pont);

  readonly vue = signal<ReglagesVue>(DEFAUTS);
  readonly ecran = signal<Ecran | null>(null);

  /** Relit les réglages et les applique à la fenêtre. */
  async charger(): Promise<void> {
    if (!this.pont.disponible) {
      this.appliquer(DEFAUTS);
      return;
    }
    const [reglages, ecran] = await Promise.all([this.pont.reglages(), this.pont.ecran()]);
    this.vue.set(reglages);
    this.ecran.set(ecran);
    this.appliquer(reglages);
  }

  /**
   * Enregistre, et prend pour vrai CE QUE RUST REND.
   *
   * Si une valeur a été ramenée dans ses bornes, l'écran doit le montrer tout
   * de suite : garder ce qu'on a envoyé laisserait un curseur à une position
   * que le fichier ne porte pas, et le joueur croirait avoir réglé 200 là où
   * le jeu en recevra 32.
   */
  async enregistrer(reglages: ReglagesVue): Promise<void> {
    if (!this.pont.disponible) {
      this.vue.set(reglages);
      this.appliquer(reglages);
      return;
    }
    const ecrits = await this.pont.enregistrerReglages(reglages);
    this.vue.set(ecrits);
    this.appliquer(ecrits);
  }

  /** Change une section, et enregistre. */
  async modifier(partiel: Partial<ReglagesVue>): Promise<void> {
    await this.enregistrer({ ...this.vue(), ...partiel });
  }

  /**
   * Pose le fond et le voile sur `<html>`.
   *
   * Deux propriétés personnalisées, et rien d'autre : la mise en forme reste
   * dans les feuilles, et ce service ne fait qu'y injecter deux valeurs.
   */
  private appliquer(reglages: ReglagesVue): void {
    const racine = document.documentElement;
    racine.dataset['fond'] = reglages.apparence.fond;
    racine.style.setProperty('--voile', String(reglages.apparence.voile));
  }
}
