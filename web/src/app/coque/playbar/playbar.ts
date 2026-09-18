import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import { Download, Play, RefreshCw, Rocket, Square, WifiOff } from '../../noyau/icones';
import * as format from '../../noyau/format';
import { Incidents } from '../../noyau/incidents';
import { Notifications } from '../../noyau/notifications';
import { Pack } from '../../noyau/pack';

/** L'allure du bouton, au sens du design system. */
type Allure = 'pret' | 'occupe' | 'inerte';

/**
 * Combien de temps le bouton attend la confirmation d'un arrêt.
 *
 * Assez pour un second clic délibéré, assez peu pour qu'un bouton oublié sur
 * « Arrêter le jeu ? » ne devienne pas un piège trois heures plus tard.
 */
const DELAI_DE_CONFIRMATION = 4000;

/**
 * LE bouton, et ce qui s'écrit au-dessus de lui.
 *
 * ## Pourquoi il vit dans la coque et non dans Spawn
 *
 * Parce que ce n'est pas une décision de la page : c'est l'état du disque et
 * celui de la session qui le pilotent, et ils sont tous les deux dans des
 * services racine. Le design system le place dans la barre du bas de la
 * fenêtre, au centre ; Spawn n'en est que la page qui l'affiche.
 *
 * ## Il ne lance plus le jeu tout seul
 *
 * Il disait « Mettre à jour et jouer », et il faisait les deux. Sam l'a repris
 * là-dessus à la recette : cliquer pour poser un modpack et voir Minecraft
 * démarrer n'est pas ce qu'on a demandé. **Dès qu'il y a quelque chose à poser,
 * le geste est de poser** ; jouer vient après, d'un second clic.
 *
 * La règle n'est pas ici : c'est `mc_pack::comparaison::a_poser`, la même que
 * suit l'installation, et l'`Action` que Rust rend en est le résultat. Ce
 * composant n'en tire qu'un libellé — sans quoi le bouton et l'installation
 * pourraient un jour ne plus être d'accord sur ce qui va se passer.
 *
 * ## Les états, et ce qui les distingue
 *
 * Ils ne viennent pas d'un seul champ : l'`Action` dit ce qu'il faut FAIRE,
 * l'écart dit ce qui sépare le posé du publié, et l'activité s'observe. Les
 * confondre ferait répondre « une opération est déjà en cours » à quelqu'un qui
 * clique pendant sa partie, laquelle est le plus long état de la session.
 *
 * ## Les deux lignes au-dessus ne sont pas décoratives
 *
 * La première dit POURQUOI le bouton est ce qu'il est — « pas encore
 * installé », « mise à jour disponible », « hors ligne » — ou, pendant le
 * travail, à quelle étape on en est.
 *
 * La seconde n'existe QUE pendant le travail, et c'est l'autre retour de
 * recette : « on sait à peu près à quelle étape on est, mais pas ce qu'on
 * télécharge, ni à quelle vitesse ». Elle porte le fichier en cours, le compte
 * de fichiers, les octets, le débit et le temps restant — toutes des données
 * que `Suivi` émet cinq fois par seconde et que personne n'affichait.
 */
@Component({
  selector: 'app-playbar',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './playbar.html',
  styleUrl: './playbar.css',
})
export class Playbar {
  private readonly pack = inject(Pack);
  private readonly incidents = inject(Incidents);
  private readonly notifications = inject(Notifications);

  protected readonly etat = this.pack.etat;
  protected readonly bouton = this.pack.bouton;
  protected readonly avancement = this.pack.avancement;
  protected readonly progression = this.pack.progression;
  protected readonly etapeCourante = this.pack.etapeCourante;

  protected readonly WifiOff = WifiOff;

  /** Prêt à cliquer, occupé, ou empêché. */
  protected readonly allure = computed<Allure>(() => {
    switch (this.bouton()) {
      case 'installer':
      case 'jouer':
        return this.empeche() ? 'inerte' : 'pret';
      case 'occupe':
      case 'inconnu':
        return 'occupe';
      // **Cliquable, et c'est un ajout de recette.** `jouer` ne rend la main
      // qu'à la fin de la partie : un Minecraft figé sur un écran de
      // chargement laissait le launcher bloqué là, sans autre issue que le
      // gestionnaire de tâches.
      case 'en-partie':
        return 'pret';
    }
  });

  /**
   * Ce qui empêche de cliquer, ou `null`.
   *
   * Hors ligne ET rien d'installé : il n'y a rien à lancer et rien à
   * télécharger. Hors ligne avec un pack posé, en revanche, se joue très bien —
   * le launcher sait seulement qu'il n'a pas pu vérifier.
   */
  protected readonly empeche = computed(() => {
    const vu = this.etat();
    if (!vu) {
      return null;
    }
    if (vu.horsLigne && !vu.installe) {
      return 'hors-ligne' as const;
    }
    return null;
  });

  /**
   * Le libellé du bouton.
   *
   * Aucun ne contient plus « et jouer » : ce que le bouton annonce est
   * exactement ce qu'il fait, et rien de plus.
   */
  protected readonly libelle = computed(() => {
    switch (this.bouton()) {
      case 'inconnu':
        return 'Vérification…';
      case 'installer':
        switch (this.etat()?.ecart) {
          case 'mise-a-jour':
            return 'Mettre à jour';
          case 'reinstallation':
            return 'Réinstaller';
          default:
            return 'Installer';
        }
      case 'jouer':
        return 'Jouer';
      case 'occupe':
        return 'Installation…';
      case 'en-partie':
        return this.arretDemande() ? 'Arrêter le jeu ?' : 'En jeu';
    }
  });

  /**
   * L'icône, quand il y en a une.
   *
   * `null` pendant le travail : la barre de remplissage tient alors la place, et
   * deux marques d'activité côte à côte ne disent rien de plus.
   */
  protected readonly icone = computed<LucideIconData | null>(() => {
    switch (this.bouton()) {
      case 'installer':
        return this.etat()?.ecart === 'absent' ? Download : RefreshCw;
      case 'jouer':
        return Play;
      case 'en-partie':
        return this.arretDemande() ? Square : Rocket;
      default:
        return null;
    }
  });

  /**
   * Ce qui s'écrit après le séparateur, dans le bouton même.
   *
   * **Le pourcentage reste affiché entre deux lots**, et c'est une correction :
   * il ne s'affichait que pendant un téléchargement actif, donc il disparaissait
   * pendant la résolution des mods, l'inspection des jars et l'installateur
   * NeoForge — c'est-à-dire pendant une bonne partie du temps. Le débit, lui,
   * n'a de sens que quand quelque chose descend.
   */
  protected readonly meta = computed(() => {
    if (!this.enInstallation()) {
      return null;
    }
    const pourcent = `${Math.round(this.progression())} %`;
    const vu = this.avancement();
    return vu?.actif && vu.debit > 0 ? `${pourcent} · ${format.debit(vu.debit)}` : pourcent;
  });

  /** La phrase du haut, et son ton. */
  protected readonly indication = computed(() => {
    if (this.empeche() === 'hors-ligne') {
      return { texte: 'Hors ligne — impossible de récupérer le pack.', danger: true };
    }

    if (this.bouton() === 'en-partie') {
      return this.arretDemande()
        ? {
            texte: 'Cliquez à nouveau pour forcer l’arrêt — la partie ne sera pas sauvegardée.',
            danger: true,
          }
        : { texte: 'Le jeu tourne. Cliquez pour l’arrêter s’il ne répond plus.', danger: false };
    }

    if (this.enInstallation()) {
      const etape = this.etapeCourante();
      const ou = etape
        ? `${etape.libelle} — étape ${etape.numero} sur ${etape.total}`
        : 'Préparation';
      const note = this.avancement()?.note;
      return { texte: note ? `${ou} · ${note}` : ou, danger: false };
    }

    const pack = this.etat();
    if (!pack) {
      return { texte: 'Lecture de ce qui est installé…', danger: false };
    }
    if (pack.horsLigne) {
      return { texte: 'Hors ligne — les mises à jour ne sont pas vérifiées.', danger: false };
    }

    switch (pack.ecart) {
      case 'absent':
        return { texte: `Pas encore installé${this.versionSuffixe()}`, danger: false };
      case 'a-jour':
        return { texte: `À jour${this.versionSuffixe()}`, danger: false };
      case 'mise-a-jour':
        return { texte: `Mise à jour disponible${this.versionSuffixe()}`, danger: false };
      case 'reinstallation':
        return {
          texte: 'Réinstallation complète — vos mondes et vos configurations sont conservés',
          danger: false,
        };
      case 'inconnu':
        return { texte: 'Vérification du pack…', danger: false };
    }
  });

  /**
   * La seconde ligne : ce qui se passe RÉELLEMENT, en ce moment.
   *
   * `null` hors travail — un bouton au repos n'a pas de détail à porter — et
   * composée de segments qui s'omettent quand ils ne sont pas connus, plutôt
   * que d'afficher « 0 o sur 0 o » ou un point médian orphelin.
   */
  protected readonly detail = computed(() => {
    const vu = this.avancement();
    if (!this.enInstallation() || !vu) {
      return null;
    }

    const segments: string[] = [];

    if (vu.fichier) {
      segments.push(nomDeFichier(vu.fichier));
    }
    if (vu.fichiersTotal > 0) {
      segments.push(`${vu.fichiers} sur ${vu.fichiersTotal} fichiers`);
    }
    if (vu.total > 0) {
      segments.push(`${format.octets(vu.octets)} sur ${format.octets(vu.total)}`);
    }
    if (vu.actif && vu.debit > 0) {
      segments.push(format.debit(vu.debit));
    }
    if (vu.restant !== null) {
      segments.push(`${format.duree(vu.restant)} restantes`);
    }

    return segments.length > 0 ? segments.join(' · ') : null;
  });

  /**
   * Le remplissage de la barre, en pourcentage.
   *
   * **Il suit la progression GLOBALE, et non celle du lot en cours.** La
   * distinction a son importance : entre deux lots, la progression d'un lot
   * saute à cent pour cent alors que l'étape travaille encore — ce qui mentait.
   * La progression globale, elle, est bornée par la phase atteinte : entre deux
   * lots elle stagne, ce qui est la vérité.
   *
   * Indéterminé tant qu'on ne sait rien du tout : pendant la vérification, ou
   * avant le premier événement d'avancement.
   */
  protected readonly remplissage = computed(() => {
    if (!this.enInstallation() || !this.avancement()) {
      return null;
    }
    const pourcent = Math.round(this.progression());
    return pourcent > 0 ? `${pourcent}%` : null;
  });

  /**
   * Un geste est-il en cours ?
   *
   * **Distinct de « le bouton tourne »** : entre l'affichage de l'écran et la
   * réponse d'`etat_du_pack()`, le bouton tourne aussi, mais rien n'est en
   * train d'être posé. Confondre les deux ferait afficher « 0 % », une étape
   * « Préparation » et une ligne de détail vide à chaque démarrage, à la place
   * de la phrase qui dit qu'on lit le disque.
   */
  private readonly enInstallation = computed(() => this.bouton() === 'occupe');

  /**
   * L'arrêt a-t-il été demandé une première fois ?
   *
   * **Deux temps plutôt qu'une modale.** Tuer le jeu fait perdre ce qui n'a pas
   * été sauvegardé, et ce bouton occupe le centre de la barre du bas : un clic
   * de trop y est vite arrivé. Une modale serait plus lourde qu'il n'y paraît —
   * il faudrait la fermer au clavier, la sortir du flux, lui donner un focus —
   * là où le bouton lui-même peut poser la question.
   *
   * La demande retombe d'elle-même : un bouton resté sur « Arrêter le jeu ? »
   * pendant une heure de partie serait un piège plutôt qu'une garde.
   */
  private readonly arretDemande = signal(false);

  /**
   * LE clic.
   *
   * Deux gestes derrière un bouton, et c'est l'`Action` de Rust qui tranche :
   * s'il y a quelque chose à poser, on pose ; sinon on joue.
   */
  protected async agir(): Promise<void> {
    if (this.allure() !== 'pret') {
      return;
    }
    if (this.bouton() === 'en-partie') {
      this.arreter();
      return;
    }
    if (this.bouton() === 'installer') {
      await this.installer();
      return;
    }
    await this.jouer();
  }

  /**
   * Premier clic : on demande. Second : on arrête.
   *
   * Le compte rendu de la partie interrompue arrivera par `jouer()`, qui est
   * toujours en vol — ce n'est pas à ce geste-ci de l'annoncer.
   */
  private arreter(): void {
    if (!this.arretDemande()) {
      this.arretDemande.set(true);
      setTimeout(() => this.arretDemande.set(false), DELAI_DE_CONFIRMATION);
      return;
    }
    this.arretDemande.set(false);
    void this.incidents.pendant(() => this.pack.arreterLeJeu());
  }

  private async installer(): Promise<void> {
    const rendu = await this.incidents.pendant(() => this.pack.installer());
    if (!rendu) {
      return;
    }
    if (rendu.introuvables.length > 0) {
      this.notifications.signaler(
        'warning',
        `${rendu.introuvables.length} mod(s) introuvable(s)`,
        rendu.introuvables.join(', '),
      );
      return;
    }
    this.notifications.signaler('success', 'Pack installé', rendu.verdict);
  }

  private async jouer(): Promise<void> {
    const rendu = await this.incidents.pendant(() => this.pack.jouer());
    if (!rendu) {
      return;
    }
    if (rendu.introuvables.length > 0) {
      this.notifications.signaler(
        'warning',
        `${rendu.introuvables.length} mod(s) introuvable(s)`,
        rendu.introuvables.join(', '),
      );
    } else if (rendu.rattrapee) {
      this.notifications.signaler('success', 'Pack mis à jour', rendu.verdict);
    }
  }

  /** « · 1.4.2 », ou rien. Sans point médian orphelin quand il n'y a pas de version. */
  private versionSuffixe(): string {
    const version = this.etat()?.version;
    return version ? ` · ${version}` : '';
  }
}

/**
 * Le nom seul d'un chemin.
 *
 * Rust peut annoncer un chemin complet, et une ligne d'information qui affiche
 * `/home/…/shared/libraries/net/neoforged/…/neoforge-21.1.250-universal.jar`
 * déborde la fenêtre et ne se lit pas. Fonction libre : elle ne dépend de rien
 * du composant, et c'est ce qui la rend éprouvable seule.
 */
export function nomDeFichier(chemin: string): string {
  const morceaux = chemin.split(/[\\/]/);
  return morceaux[morceaux.length - 1] || chemin;
}
