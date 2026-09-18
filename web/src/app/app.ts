import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { Amorce } from './amorce/amorce';
import { BarreTitre } from './coque/barre-titre/barre-titre';
import { Menu } from './coque/menu/menu';
import { Overlay } from './coque/overlay/overlay';
import { Fenetre } from './noyau/fenetre';
import { Incidents } from './noyau/incidents';
import { Marque } from './noyau/marque';
import { Pont } from './noyau/pont';
import { Reglages } from './noyau/reglages';
import { Session } from './noyau/session';

/**
 * Le plancher de l'écran de démarrage, en millisecondes.
 *
 * Neuf cents : assez pour qu'on ait le temps de lire la phrase, assez peu pour
 * qu'on ne le remarque pas comme une attente. Un plancher est nécessaire parce
 * que sans lui, un démarrage rapide ferait clignoter l'amorce pendant deux
 * images — ce qui se remarque bien plus qu'une seconde tenue.
 */
const PLANCHER_AMORCE = 900;

/**
 * La coque : le fond, la barre de titre, le menu, et la page.
 *
 * ## La charpente, et ce qu'elle doit au fond
 *
 * Le fond plein cadre vit sur `body`, hors de tout composant. C'est la SEULE
 * surface qui couvre la fenêtre entière, barre de titre comprise, et c'est
 * pourquoi rien ici ne porte d'image : tout ce qui est ici se pose dessus.
 *
 * ## L'amorce ne couvre pas la barre de titre
 *
 * Elle occupe la zone de contenu, sous la barre. `statut()` enchaîne deux
 * allers-retours réseau : sur un réseau lent ou derrière un portail captif,
 * une amorce plein écran laisserait une fenêtre sans bouton système — on les a
 * retirés — et sans bouton applicatif — ils seraient dessous. Recette :
 * ouvrir le build packagé SANS RÉSEAU, et fermer pendant l'amorce.
 */
@Component({
  selector: 'app-root',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterOutlet, Amorce, BarreTitre, Menu, Overlay],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App {
  // Tous injectés en CHAMPS et non dans `demarrer()` : `inject()` n'est
  // utilisable que dans un contexte d'injection, et une méthode asynchrone en
  // sort dès le premier `await`. L'erreur ne se voit qu'à l'exécution, sur un
  // NG0203 qui ne nomme pas la ligne fautive.
  private readonly pont = inject(Pont);
  private readonly session = inject(Session);
  private readonly incidents = inject(Incidents);
  private readonly marque = inject(Marque);
  private readonly reglages = inject(Reglages);
  private readonly fenetre = inject(Fenetre);

  /** Faux hors de la fenêtre Tauri : l'écran le dit plutôt que d'échouer. */
  protected readonly disponible = this.pont.disponible;

  /** L'amorce est-elle encore affichée ? */
  protected readonly amorce = signal(true);

  protected readonly incidentOuvert = this.incidents.ouvert;

  /**
   * Le contenu est flouté quand un incident est ouvert.
   *
   * L'attribut porte sur la ZONE DE CONTENU et non sur la coque entière :
   * flouter la coque flouterait l'overlay avec elle, puisqu'il en est un
   * descendant — un élément ne peut pas se flouter sans se flouter.
   */
  protected readonly flou = computed(() => (this.incidentOuvert() ? '' : null));

  constructor() {
    void this.demarrer();
  }

  /**
   * Ce qui se passe pendant l'amorce.
   *
   * Tout en parallèle : la marque, les réglages, l'état de la fenêtre et la
   * session partent ensemble. Les enchaîner ferait de l'amorce la somme de
   * quatre latences au lieu de la plus grande.
   *
   * `finally` et non la seule branche de succès : si `statut()` échoue — réseau
   * coupé, jeton illisible — l'amorce doit s'effacer quand même, sinon le
   * launcher reste bloqué sur son écran de démarrage sans une erreur visible.
   */
  private async demarrer(): Promise<void> {
    const plancher = new Promise((suite) => setTimeout(suite, PLANCHER_AMORCE));

    try {
      await Promise.all([
        this.marque.charger(),
        this.reglages.charger(),
        this.fenetre.observer(),
        this.session.ouvrir(),
      ]);
    } catch (cause) {
      this.incidents.signaler(cause);
    } finally {
      await plancher;
      this.amorce.set(false);
    }
  }
}
