import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { Router } from '@angular/router';
import { LucideAngularModule } from 'lucide-angular';

import { Check, Copy, ExternalLink, TriangleAlert } from '../noyau/icones';
import { Incidents } from '../noyau/incidents';
import { Notifications } from '../noyau/notifications';
import { Fenetre } from '../noyau/fenetre';
import { Pont } from '../noyau/pont';
import { Session } from '../noyau/session';

/** Où l'on en est, des trois pas du design system. */
type Pas = 'inviter' | 'code' | 'sans-licence';

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

  protected readonly compte = this.session.compte;
  protected readonly code = this.session.code;
  protected readonly occupe = this.session.occupe;

  protected readonly Copy = Copy;
  protected readonly Check = Check;
  protected readonly ExternalLink = ExternalLink;
  protected readonly TriangleAlert = TriangleAlert;

  protected readonly pas = computed<Pas>(() => {
    if (this.session.sansLicence()) {
      return 'sans-licence';
    }
    return this.code() ? 'code' : 'inviter';
  });

  /** Combien des trois segments sont franchis. */
  protected readonly franchis = computed(() => (this.pas() === 'code' ? 2 : 1));

  protected async connecter(): Promise<void> {
    await this.incidents.pendant(async () => {
      await this.session.connecter();
      if (!this.session.jouable()) {
        // Compte Microsoft valide, mais sans licence : on reste ici, et l'écran
        // le dit. C'est le quatrième état de la page.
        return;
      }
      this.notifications.signaler('success', 'Connecté', this.compte()?.pseudo ?? null);

      // Dans la fenêtre de connexion, on ne NAVIGUE pas : on rend la main à la
      // fenêtre principale, qui se montre, relit sa session et va à Spawn. Ici,
      // il n'y a rien après — la fenêtre se ferme.
      await this.pont.connexionReussie();

      // Hors de Tauri — un navigateur devant le serveur de développement — il
      // n'y a qu'un onglet : la commande ci-dessus n'a rien fait, et c'est le
      // routeur qui emmène.
      if (!this.fenetre.dansUneFenetreDediee) {
        await this.router.navigate(['/spawn']);
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
