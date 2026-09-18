import {
  ChangeDetectionStrategy,
  Component,
  afterNextRender,
  computed,
  inject,
  signal,
} from '@angular/core';
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
 * Le plancher de l'amorce Angular, en millisecondes.
 *
 * Il valait neuf cents quand cette amorce était le SEUL écran de démarrage :
 * il fallait alors couvrir tout le chargement d'Angular, et une durée tenue se
 * remarque moins qu'un clignotement de deux images.
 *
 * Depuis qu'une vraie fenêtre d'écran de démarrage couvre ce chargement — voir
 * `crates/mc-app/src/demarrage.rs` — cette amorce-ci ne couvre plus que
 * l'attente du RÉSEAU, après que la fenêtre s'est affichée. La faire durer
 * neuf cents millisecondes de plus reviendrait à ajouter une attente à une
 * attente.
 *
 * Quatre cents reste un plancher, parce qu'il en faut un : sans lui, une
 * session déjà en cache ferait clignoter l'amorce le temps d'une image.
 */
const PLANCHER_AMORCE = 400;

/**
 * Le délai entre le premier rendu et le signal envoyé à Rust.
 *
 * `afterNextRender` se déclenche quand Angular a écrit dans le DOM — pas
 * quand le navigateur a PEINT. Montrer la fenêtre à cet instant précis la
 * ferait apparaître sur une image encore vide, ce qui remplacerait un écran de
 * démarrage propre par un clignotement.
 *
 * Deux cent cinquante millisecondes couvrent plusieurs images à soixante hertz,
 * y compris le décodage de l'image de fond. C'est le délai que Sam a proposé,
 * et il est juste : plus court, on voit le vide ; plus long, on attend pour
 * rien.
 */
const AVANT_DE_MONTRER = 250;

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
    // Le signal qui referme l'écran de démarrage et montre la fenêtre.
    //
    // Il ne dépend PAS de `demarrer()`, et c'est délibéré : celui-ci interroge
    // le réseau, ce qui peut durer derrière un portail captif. Attendre ses
    // données pour montrer la fenêtre garderait le joueur devant un écran de
    // démarrage sans le moindre bouton — exactement ce qu'on refuse par
    // ailleurs en gardant la barre de titre au-dessus de l'amorce.
    //
    // La fenêtre apparaît donc dès que l'interface est dessinée, et l'amorce
    // Angular prend le relais pendant l'attente du réseau. Les deux écrans
    // sont identiques à l'œil : le passage ne se voit pas.
    afterNextRender(() => {
      setTimeout(() => void this.pont.frontPret().catch(() => {}), AVANT_DE_MONTRER);
    });

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
