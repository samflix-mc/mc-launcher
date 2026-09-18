import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import { Download, Play, RefreshCw, Rocket, WifiOff } from '../../noyau/icones';
import * as format from '../../noyau/format';
import { Incidents } from '../../noyau/incidents';
import { Notifications } from '../../noyau/notifications';
import { Pack } from '../../noyau/pack';

/** L'allure du bouton, au sens du design system. */
type Allure = 'pret' | 'occupe' | 'inerte';

/**
 * LE bouton, et la phrase au-dessus de lui.
 *
 * ## Pourquoi il vit dans la coque et non dans Spawn
 *
 * Parce que ce n'est pas une décision de la page : c'est l'état du disque et
 * celui de la session qui le pilotent, et ils sont tous les deux dans des
 * services racine. Le design system le place dans la barre du bas de la
 * fenêtre, au centre ; Spawn n'en est que la page qui l'affiche.
 *
 * ## Les six états, et ce qui les distingue
 *
 * Le design system en dessine six : installer, jouer, mettre à jour et jouer,
 * installation en cours, empêché faute de réseau, empêché pour une autre
 * raison. Ils ne viennent pas d'un seul champ — l'`Action` que Rust rend dit ce
 * que LE DISQUE impose, l'écart dit ce qui sépare le posé du publié, et
 * l'activité s'observe. Les confondre ferait répondre « une installation est
 * déjà en cours » à quelqu'un qui clique pendant sa partie.
 *
 * ## La phrase au-dessus n'est pas décorative
 *
 * C'est elle qui dit POURQUOI le bouton est ce qu'il est : « le pack n'est pas
 * installé », « une mise à jour est disponible », « hors ligne ». Sans elle,
 * l'écran demande de deviner — ce qui était exactement le reproche fait à
 * l'ancienne page Spawn.
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
      case 'en-partie':
        return 'inerte';
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

  /** Le libellé du bouton. */
  protected readonly libelle = computed(() => {
    switch (this.bouton()) {
      case 'inconnu':
        return 'Vérification…';
      case 'installer':
        return 'Installer';
      case 'jouer':
        switch (this.etat()?.ecart) {
          case 'mise-a-jour':
            return 'Mettre à jour et jouer';
          case 'reinstallation':
            return 'Réinstaller et jouer';
          default:
            return 'Jouer';
        }
      case 'occupe':
        return 'Installation…';
      case 'en-partie':
        return 'En jeu';
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
        return Download;
      case 'jouer':
        return this.etat()?.ecart === 'a-jour' || this.etat()?.ecart === 'inconnu'
          ? Play
          : RefreshCw;
      case 'en-partie':
        return Rocket;
      default:
        return null;
    }
  });

  /**
   * Ce qui s'écrit après le séparateur, dans le bouton même.
   *
   * Le pourcentage et le débit pendant le travail, et rien autrement : un
   * bouton au repos n'a pas de chiffre à porter.
   */
  protected readonly meta = computed(() => {
    const vu = this.avancement();
    if (!vu || !vu.actif) {
      return null;
    }
    return `${Math.round(this.progression())} % · ${format.debit(vu.debit)}`;
  });

  /** La phrase au-dessus du bouton, et son ton. */
  protected readonly indication = computed(() => {
    if (this.empeche() === 'hors-ligne') {
      return { texte: 'Hors ligne — impossible de récupérer le pack.', danger: true };
    }

    const vu = this.avancement();
    if (this.bouton() === 'en-partie') {
      return { texte: 'Le jeu tourne. Le launcher attend sa fin.', danger: false };
    }
    if (this.bouton() === 'occupe' && vu) {
      const etape = this.etapeCourante();
      const ou = etape ? `${etape.libelle} — ${etape.numero} sur ${etape.total}` : 'Préparation';
      return {
        texte: vu.actif ? `${ou} · ${format.octets(vu.octets)} sur ${format.octets(vu.total)}` : ou,
        danger: false,
      };
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
        return {
          texte: `Mise à jour disponible${this.versionSuffixe()} — appliquée au lancement`,
          danger: false,
        };
      case 'reinstallation':
        return {
          texte: 'Réinstallation complète — vos mondes et vos configurations sont conservés',
          danger: false,
        };
      case 'inconnu':
        return { texte: 'Vérification du pack…', danger: false };
    }
  });

  /** La progression, ou `null` quand elle est indéterminée. */
  protected readonly remplissage = computed(() => {
    if (this.bouton() === 'inconnu') {
      return null;
    }
    const vu = this.avancement();
    return vu?.actif ? `${Math.round(this.progression())}%` : null;
  });

  protected async jouer(): Promise<void> {
    if (this.allure() !== 'pret') {
      return;
    }
    const partie = await this.incidents.pendant(() => this.pack.jouer());
    if (!partie) {
      return;
    }
    if (partie.introuvables.length > 0) {
      this.notifications.signaler(
        'warning',
        `${partie.introuvables.length} mod(s) introuvable(s)`,
        partie.introuvables.join(', '),
      );
    } else if (partie.rattrapee) {
      this.notifications.signaler('success', 'Pack mis à jour', partie.verdict);
    }
  }

  /** « · 1.4.2 », ou rien. Sans point médian orphelin quand il n'y a pas de version. */
  private versionSuffixe(): string {
    const version = this.etat()?.version;
    return version ? ` · ${version}` : '';
  }
}
