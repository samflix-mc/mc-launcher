import {
  ChangeDetectionStrategy,
  Component,
  afterNextRender,
  computed,
  inject,
  signal,
} from '@angular/core';
import { NavigationEnd, Router, RouterOutlet, type ActivatedRouteSnapshot } from '@angular/router';

import { Amorce } from './amorce/amorce';
import { BarreTitre } from './coque/barre-titre/barre-titre';
import { Incident } from './coque/incident/incident';
import { Joueur } from './coque/joueur/joueur';
import { Nav } from './coque/nav/nav';
import { Notifications as PanneauNotifications } from './coque/notifications/notifications';
import { Playbar } from './coque/playbar/playbar';
import { Fenetre } from './noyau/fenetre';
import { Incidents } from './noyau/incidents';
import { Marque } from './noyau/marque';
import { Pack } from './noyau/pack';
import { Pont } from './noyau/pont';
import { Reglages } from './noyau/reglages';
import { Session } from './noyau/session';

/**
 * Le plancher de l'amorce Angular, en millisecondes.
 *
 * Depuis qu'une vraie fenêtre d'écran de démarrage couvre le chargement — voir
 * `crates/mc-app/src/demarrage.rs` — cette amorce-ci ne couvre plus que
 * l'attente du RÉSEAU, après que la fenêtre s'est affichée. Quatre cents reste
 * un plancher, parce qu'il en faut un : sans lui, une session déjà en cache
 * ferait clignoter l'amorce le temps d'une image.
 */
const PLANCHER_AMORCE = 400;

/**
 * Le délai entre le premier rendu et le signal envoyé à Rust.
 *
 * `afterNextRender` se déclenche quand Angular a écrit dans le DOM — pas quand
 * le navigateur a PEINT. Montrer la fenêtre à cet instant précis la ferait
 * apparaître sur une image encore vide, ce qui remplacerait un écran de
 * démarrage propre par un clignotement.
 */
const AVANT_DE_MONTRER = 250;

/** Ce que la barre du bas porte, selon la page. Voir `routes.ts`. */
export type Bas = 'jouer' | 'joueur' | 'aucune';

/** La route qui vit dans sa propre fenêtre. */
const ROUTE_CONNEXION = '/connexion';

/**
 * La coque : la fenêtre, la scène, la barre de titre, la page.
 *
 * ## La charpente vient du design system, telle quelle
 *
 * `.hm-window` contient `.hm-stage` — l'image et son dégradé de lisibilité —
 * puis `.hm-titlebar`, qui flotte par-dessus, puis `.hm-page`, dont les trois
 * rangs sont la pilule de navigation centrée, le contenu, et la barre du bas.
 * Le MILIEU de la page est laissé vide à dessein : c'est par là que l'image
 * passe, et c'est le seul endroit de l'écran où elle se voit vraiment.
 *
 * ## L'amorce ne couvre pas la barre de titre
 *
 * Elle occupe la zone de contenu, sous la barre. `statut()` enchaîne deux
 * allers-retours réseau : sur un réseau lent ou derrière un portail captif, une
 * amorce plein écran laisserait une fenêtre sans bouton système — on les a
 * retirés — et sans bouton applicatif — ils seraient dessous. Recette : ouvrir
 * le build packagé SANS RÉSEAU, et fermer pendant l'amorce.
 *
 * ## Tant que la session n'est pas jouable, il n'y a pas de coque
 *
 * Ni pilule de navigation, ni barre du bas, ni badge joueur : la page de
 * connexion est une modale par-dessus la scène, et rien derrière elle n'est
 * atteignable. Montrer un menu et un bouton « se déconnecter » à quelqu'un qui
 * n'est pas connecté était le premier reproche de la recette.
 */
@Component({
  selector: 'app-root',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterOutlet, Amorce, BarreTitre, Nav, Joueur, Playbar, Incident, PanneauNotifications],
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
  private readonly marqueService = inject(Marque);
  private readonly reglages = inject(Reglages);
  private readonly fenetre = inject(Fenetre);
  private readonly pack = inject(Pack);
  private readonly router = inject(Router);

  /** Faux hors de la fenêtre Tauri ET hors du serveur de développement. */
  protected readonly disponible = this.pont.disponible;

  /** L'amorce est-elle encore affichée ? */
  protected readonly amorce = signal(true);

  protected readonly marque = this.marqueService.vue;
  protected readonly maximisee = this.fenetre.maximisee;
  protected readonly jouable = this.session.jouable;

  /**
   * Sommes-nous sur la page de connexion ?
   *
   * Elle ne se dessine pas comme les autres : c'est une FENÊTRE à elle, de
   * quatre cent quarante pixels, avec une feuille dépolie pleine surface et une
   * barre de titre sans bouton d'agrandissement. La coque — navigation, bouton
   * de jeu, badge joueur — n'y existe pas.
   */
  protected readonly surConnexion = signal(false);

  /**
   * Ce que la barre du bas porte, lu sur la route courante.
   *
   * Dans les DONNÉES de la route et non dans un test sur l'URL : une chaîne
   * comparée à `'/spawn'` se casse le jour où une route gagne un paramètre, et
   * le symptôme est une barre du bas vide que rien n'explique.
   */
  protected readonly bas = signal<Bas>('aucune');

  /** Vrai quand la page occupe les trois rangs, faux quand elle en occupe deux. */
  protected readonly troisRangs = computed(() => this.bas() !== 'aucune');

  constructor() {
    this.router.events.subscribe((evenement) => {
      if (!(evenement instanceof NavigationEnd)) {
        return;
      }
      this.bas.set(this.basDe(this.router.routerState.snapshot.root));

      const connexion = evenement.urlAfterRedirects.startsWith(ROUTE_CONNEXION);
      this.surConnexion.set(connexion);

      // La fenêtre PRINCIPALE ne montre jamais la connexion : elle la
      // délègue à une fenêtre dédiée, et s'efface derrière. Elle reste sur
      // cette route — personne ne la voit, puisqu'elle est cachée — et
      // reprendra la main sur le signal de session.
      //
      // La fenêtre de connexion, elle, EST déjà sur cette route : elle
      // n'ouvrirait qu'elle-même.
      if (connexion && this.fenetre.estPrincipale) {
        void this.pont.ouvrirConnexion().catch(() => {});
      }
    });

    // Quand la session s'ouvre dans l'autre fenêtre, celle-ci doit l'apprendre :
    // son service de session porte un compte nul depuis son chargement, et rien
    // ne le lui dirait.
    void this.pont.surSessionOuverte(() => void this.reprendreLaMain()).catch(() => {});

    // Le signal qui referme l'écran de démarrage et montre la fenêtre.
    //
    // Il ne dépend PAS de `demarrer()`, et c'est délibéré : celui-ci interroge
    // le réseau, ce qui peut durer derrière un portail captif. Attendre ses
    // données pour montrer la fenêtre garderait le joueur devant un écran de
    // démarrage sans le moindre bouton.
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
        this.marqueService.charger(),
        this.reglages.charger(),
        this.fenetre.observer(),
        this.session.ouvrir(),
        // L'état du pack est de la COQUE et non de Spawn : la barre de titre en
        // tire le nom du modpack, et le bouton de jeu tout le reste. L'ouvrir
        // depuis Spawn faisait disparaître le nom dès qu'on changeait de page.
        this.pack.ouvrir(),
      ]);
    } catch (cause) {
      this.incidents.signaler(cause);
    } finally {
      await plancher;
      this.amorce.set(false);
    }
  }

  /**
   * La session vient de s'ouvrir dans la fenêtre de connexion.
   *
   * On relit le compte, puis on va à Spawn : la fenêtre principale était restée
   * sur `/connexion` pendant qu'elle était cachée, et l'y laisser lui ferait
   * afficher une page de connexion à quelqu'un qui vient de se connecter.
   */
  private async reprendreLaMain(): Promise<void> {
    await this.incidents.pendant(async () => {
      await this.session.ouvrir();
      await this.pack.rafraichir();
      await this.router.navigate(['/spawn']);
    });

    // On ne dit PAS ici qu'on est prêt : c'est l'accueil qui le dira, depuis
    // son `afterNextRender`. `navigate` rend la main quand la route est
    // activée, ce qui précède le premier pixel — et Rust montrerait alors une
    // fenêtre encore vide.
    //
    // Si la navigation a échoué, personne ne le dira : le délai de garde de
    // `connexion_reussie` bascule au bout de huit secondes plutôt que de
    // laisser un « Connecté » qui ne mène nulle part.
  }

  /** La donnée `bas` de la route la plus profonde, ou « aucune » à défaut. */
  private basDe(racine: ActivatedRouteSnapshot): Bas {
    let noeud: ActivatedRouteSnapshot | undefined = racine;
    let trouve: Bas = 'aucune';
    while (noeud) {
      const valeur = noeud.data['bas'] as Bas | undefined;
      if (valeur) {
        trouve = valeur;
      }
      noeud = noeud.firstChild ?? undefined;
    }
    return trouve;
  }
}
