import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { Router } from '@angular/router';
import { LucideAngularModule } from 'lucide-angular';

import { Check, Copy, ExternalLink, TriangleAlert } from '../noyau/icones';
import { Incidents } from '../noyau/incidents';
import { Journal } from '../noyau/journal';
import { Notifications } from '../noyau/notifications';
import { Fenetre } from '../noyau/fenetre';
import { Pont } from '../noyau/pont';
import { Session } from '../noyau/session';

/** Où l'on en est, des trois pas du design system. */
type Pas = 'inviter' | 'code' | 'fait' | 'sans-licence';

/**
 * Ouvrir une session Microsoft — une modale, et rien derrière.
 *
 * ## Elle est BLOQUANTE, et c'est le premier reproche de la recette
 *
 * Avant, la page de connexion s'affichait dans la coque : un menu à gauche, un
 * bouton « se déconnecter », un badge joueur vide. On proposait de se
 * déconnecter à quelqu'un qui n'était pas connecté. Maintenant la coque
 * n'existe pas tant que la session n'est pas jouable — voir `app.html` — et
 * cette page est un `.hm-scrim` par-dessus la scène, avec un seul dialogue.
 *
 * ## Les trois pas, et le quatrième qui n'en est pas un
 *
 * Le design system en dessine trois : inviter, saisir le code, c'est fait. Le
 * troisième ne s'affiche pas ici : quand la session devient jouable, la garde
 * du routeur emmène vers Spawn, et un pas « c'est fait » qu'on ne voit qu'un
 * dixième de seconde ne vaut pas d'être écrit.
 *
 * Le quatrième état, lui, n'est pas un pas : un compte Microsoft valide qui
 * n'a jamais acheté Minecraft. Les deux gardes du routeur lisent le MÊME
 * prédicat — `jouable()` — justement pour que ce cas ne rebondisse pas entre
 * elles : il reste ici, et l'écran le dit.
 *
 * ## Les quatre secondes
 *
 * Entre le moment où le joueur autorise chez Microsoft et celui où le launcher
 * s'en aperçoit, il s'écoule un intervalle de sondage — mesuré à environ quatre
 * secondes. C'est ce que `.hm-auth__wait` habite : un rond qui tourne et une
 * phrase, pour que l'attente soit annoncée plutôt que subie.
 */
@Component({
  selector: 'app-connexion',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './connexion.html',
  styleUrl: './connexion.css',
})
export class Connexion {
  private readonly session = inject(Session);
  private readonly incidents = inject(Incidents);
  private readonly notifications = inject(Notifications);
  private readonly pont = inject(Pont);
  private readonly fenetre = inject(Fenetre);
  private readonly router = inject(Router);
  private readonly trace = inject(Journal);

  protected readonly compte = this.session.compte;
  protected readonly code = this.session.code;
  protected readonly occupe = this.session.occupe;

  protected readonly Copy = Copy;
  protected readonly Check = Check;
  protected readonly ExternalLink = ExternalLink;
  protected readonly TriangleAlert = TriangleAlert;

  /**
   * L'écran « c'est fait », posé à la main.
   *
   * Il ne se DÉDUIT pas de la session : au moment où elle devient jouable, tout
   * le reste est prêt à basculer, et un état qui disparaîtrait aussitôt ne se
   * lirait pas. C'est un signal qu'on lève, et que Rust tient une seconde et
   * demie au moins — voir `PLANCHER_CONNECTE` dans `fenetres.rs`.
   */
  private readonly abouti = signal(false);

  protected readonly pas = computed<Pas>(() => {
    if (this.session.sansLicence()) {
      return 'sans-licence';
    }
    if (this.abouti()) {
      return 'fait';
    }
    return this.code() ? 'code' : 'inviter';
  });

  /** Combien des trois segments sont franchis. */
  protected readonly franchis = computed(() => {
    switch (this.pas()) {
      case 'fait':
        return 3;
      case 'code':
        return 2;
      default:
        return 1;
    }
  });

  protected async connecter(): Promise<void> {
    this.trace.etape('connexion demandée : appel de « connexion » chez Rust');
    await this.incidents.pendant(async () => {
      await this.session.connecter();
      this.trace.etape(
        `Microsoft a répondu — jouable=${this.session.jouable()}, ` +
          `sansLicence=${this.session.sansLicence()}`,
      );
      if (!this.session.jouable()) {
        // Compte Microsoft valide, mais sans licence : on reste ici, et l'écran
        // le dit. C'est le quatrième état de la page.
        return;
      }
      this.notifications.signaler('success', 'Connecté', this.compte()?.pseudo ?? null);

      // L'écran « c'est fait » AVANT la bascule : sans lui, la connexion
      // réussit et la fenêtre disparaît dans la même image, ce qui se lit comme
      // un plantage plutôt que comme une réussite.
      this.abouti.set(true);
      this.trace.etape('étape « Connecté » affichée');

      // Dans la fenêtre de connexion, on ne NAVIGUE pas : on rend la main à la
      // fenêtre principale, qui se montre, relit sa session et va à Spawn. Ici,
      // il n'y a rien après — la fenêtre se ferme.
      await this.pont.connexionReussie();
      this.trace.etape('« connexion_reussie » a rendu la main');

      // Hors de Tauri — un navigateur devant le serveur de développement — il
      // n'y a qu'un onglet : la commande ci-dessus n'a rien fait, et c'est le
      // routeur qui emmène.
      if (!this.fenetre.dansUneFenetreDediee) {
        this.trace.etape('hors fenêtre dédiée : le routeur emmène vers Spawn');
        await this.router.navigate(['/spawn']);
      } else {
        this.trace.detail('fenêtre dédiée : aucune navigation, Rust ferme la fenêtre');
      }
    });
  }

  protected async deconnecter(): Promise<void> {
    await this.incidents.pendant(() => this.session.deconnecter());
  }

  /** Recopier le code plutôt que de le retaper. */
  protected async copier(code: string): Promise<void> {
    await navigator.clipboard.writeText(code).catch(() => {});
  }

  /**
   * Rouvre la page Microsoft.
   *
   * Rust l'a déjà tentée au moment où le code est arrivé ; ceci est le recours,
   * pour le cas où aucun navigateur n'était réglé par défaut.
   */
  protected async ouvrirLaPage(url: string): Promise<void> {
    await this.incidents.pendant(() => this.pont.ouvrirPage(url));
  }
}
