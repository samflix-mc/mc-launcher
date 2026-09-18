import {
  ChangeDetectionStrategy,
  Component,
  afterNextRender,
  computed,
  inject,
} from '@angular/core';
import { LucideAngularModule } from 'lucide-angular';

import { Check, Clock, Globe, Package, TriangleAlert, Users } from '../noyau/icones';
import { CarteNouvelle } from '../nouvelles/carte/carte-nouvelle';
import { Fenetre } from '../noyau/fenetre';
import { Incidents } from '../noyau/incidents';
import { Journal } from '../noyau/journal';
import { Nouvelles } from '../noyau/nouvelles';
import { Pack } from '../noyau/pack';
import { Pont } from '../noyau/pont';

/** Ce que la pastille d'état du pack dit, et de quelle couleur. */
interface Pastille {
  readonly variante: 'online' | 'update' | 'warning' | 'offline' | 'unknown';
  readonly libelle: string;
  readonly valeur: string | null;
}

/**
 * L'écran principal — « Spawn ».
 *
 * Le nom vient de Sam : « accueil » ne disait rien d'un launcher Minecraft, et
 * « Spawn » est l'endroit où l'on arrive. Il est employé tel quel dans la
 * navigation, dans les routes et ici, sans traduction intermédiaire.
 *
 * ## Ce que la page porte, et ce qu'elle ne porte plus
 *
 * Le design system pose deux colonnes en haut — la nouvelle épinglée à gauche,
 * les panneaux d'état à droite — et LAISSE LE MILIEU VIDE : c'est par là que
 * l'image passe, et c'est le seul endroit de l'écran où elle se voit vraiment.
 *
 * Le bouton de jeu et le badge joueur ne sont plus ici : ils vivent dans la
 * barre du bas de la fenêtre, qui est de la coque. Cette page ne décide donc
 * plus rien de ce qui se lance ; elle dit ce qui EST.
 *
 * ## La cinématique n'est plus ici non plus
 *
 * Elle y était pendant le travail, dans un panneau sous celui du modpack, et
 * elle en a été retirée à la recette. Le motif est juste : le bouton porte déjà
 * la progression globale, son pourcentage et son débit, et la phrase au-dessus
 * de lui NOMME l'étape en cours. La liste des onze phases disait donc une
 * troisième fois ce que deux éléments disaient déjà — au prix d'un panneau qui
 * apparaissait et disparaissait sous le regard, à l'endroit même où l'on suit
 * l'avancement.
 *
 * ## Pourquoi l'ancienne version était illisible
 *
 * Elle empilait sept blocs conditionnels dans une colonne : des pastilles, une
 * cinématique, une jauge, trois alertes, un verdict, une carte de nouvelle, un
 * bouton. Aucun n'avait de place réservée, donc tout bougeait ; et rien ne
 * disait à quoi chaque bloc répondait. Ici, chaque panneau a une place fixe et
 * un intitulé, et ce qui n'a rien à dire ne s'affiche pas — sans déplacer le
 * reste.
 */
@Component({
  selector: 'app-spawn',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [CarteNouvelle, LucideAngularModule],
  templateUrl: './spawn.html',
  styleUrl: './spawn.css',
})
export class Spawn {
  private readonly pack = inject(Pack);
  private readonly incidents = inject(Incidents);
  private readonly nouvelles = inject(Nouvelles);
  private readonly pont = inject(Pont);
  private readonly fenetre = inject(Fenetre);
  private readonly trace = inject(Journal);

  protected readonly etat = this.pack.etat;
  protected readonly bilan = this.pack.dernierCompteRendu;

  protected readonly epinglee = this.nouvelles.epinglee;

  protected readonly Package = Package;
  protected readonly Globe = Globe;
  protected readonly Users = Users;

  /**
   * L'état du serveur — **un emplacement réservé**, et rien de plus.
   *
   * Le design system pose un panneau « Serveur » à côté de celui du modpack :
   * une pastille, une adresse, un nombre de joueurs, un ping. Rien ne les
   * mesure encore — le launcher ne sonde aucun serveur, et le manifeste du pack
   * n'en publie pas l'adresse.
   *
   * Il est donc dessiné dans l'état que le design system réserve à l'avant-
   * première-réponse : `--unknown`, qui pulse pour dire qu'il attend. Les
   * valeurs sont des tirets.
   *
   * **Écrire « En ligne · 42/120 » en dur serait pire que de ne rien
   * afficher** : un joueur croirait le serveur joignable, et ne comprendrait
   * pas que le jeu le refuse. Un emplacement qui dit qu'il ne sait pas ne ment
   * à personne.
   *
   * Ce qu'il faudra pour le remplir : une adresse dans le manifeste, une sonde
   * côté Rust — le protocole de statut de Minecraft, une poignée de main TCP et
   * un JSON — et un rafraîchissement toutes les trente secondes, comme le
   * design system le prescrit.
   */
  protected readonly serveur = computed(() => ({
    nom: this.etat()?.nom ?? 'Serveur',
    etat: 'État inconnu',
    adresse: '—',
    joueurs: '—',
  }));
  protected readonly Check = Check;
  protected readonly Clock = Clock;
  protected readonly TriangleAlert = TriangleAlert;

  /** L'état du pack, en une pastille. */
  protected readonly pastille = computed<Pastille>(() => {
    const vu = this.etat();
    if (!vu) {
      return { variante: 'unknown', libelle: 'Vérification', valeur: null };
    }
    if (vu.horsLigne) {
      return { variante: 'offline', libelle: 'Hors ligne', valeur: vu.version };
    }
    switch (vu.ecart) {
      case 'a-jour':
        return { variante: 'online', libelle: 'À jour', valeur: vu.version };
      case 'mise-a-jour':
        return { variante: 'update', libelle: 'Mise à jour', valeur: vu.version };
      case 'reinstallation':
        return { variante: 'update', libelle: 'Réinstallation', valeur: vu.version };
      case 'absent':
        return { variante: 'warning', libelle: 'Pas installé', valeur: vu.version };
      case 'inconnu':
        return { variante: 'unknown', libelle: 'Vérification', valeur: vu.version };
    }
  });

  protected readonly introuvables = computed(() => this.bilan()?.introuvables ?? []);
  protected readonly ecarts = computed(() => this.bilan()?.ecarts ?? []);
  protected readonly purge = computed(() => this.bilan()?.purge ?? []);

  /** Y a-t-il quelque chose à dire du dernier geste ? */
  protected readonly compteRendu = computed(
    () =>
      this.bilan() !== null &&
      (this.introuvables().length > 0 || this.ecarts().length > 0 || this.purge().length > 0),
  );

  constructor() {
    // **C'est d'ICI que part le signal qui montre la fenêtre principale.**
    //
    // Depuis `afterNextRender`, et pas depuis la navigation : `navigate` rend
    // la main quand la route est ACTIVÉE, ce qui précède le premier pixel de
    // plusieurs images. Montrer la fenêtre à cet instant-là la ferait
    // apparaître sur un écran encore vide — exactement le défaut qu'on cherche
    // à supprimer.
    //
    // Rust n'attend ce signal que pendant une connexion ; au démarrage
    // ordinaire, il ne l'écoute pas, et l'envoyer ne coûte rien.
    //
    // **Mais seule la fenêtre principale a le droit de le dire.** Une fenêtre
    // dédiée qui monterait cette page — ce qui ne devrait plus arriver, mais
    // arrivait — annoncerait à Rust que la PRINCIPALE est prête alors qu'elle
    // n'a rien rendu du tout : Rust basculerait aussitôt, et l'on retomberait
    // sur l'écran qui se remplit sous les yeux.
    afterNextRender(() => {
      if (this.fenetre.dansUneFenetreDediee) {
        this.trace.souci(
          'Spawn monté dans une fenêtre DÉDIÉE : « principale_prete » n’est pas envoyé',
        );
        return;
      }
      this.trace.etape('Spawn dessiné : « principale_prete » envoyé');
      void this.pont.principalePrete().catch(() => {});
    });

    // Le pack est ouvert par la COQUE — la barre de titre et le bouton de jeu
    // en dépendent, et ils survivent à cette page. On se contente de redemander
    // l'état : le disque a pu changer pendant qu'on était ailleurs.
    void this.incidents.pendant(() => this.pack.rafraichir());
    // Le fil ne fait pas partie de ce que l'écran attend : une tuile vide vaut
    // mieux qu'un écran qui attend le réseau pour montrer son bouton.
    void this.nouvelles.charger().catch(() => {});
  }
}
