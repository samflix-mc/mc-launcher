import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { Router } from '@angular/router';
import { LucideAngularModule } from 'lucide-angular';

import { ChevronDown, LogOut, RefreshCw, User } from '../../noyau/icones';
import { Incidents } from '../../noyau/incidents';
import { Notifications } from '../../noyau/notifications';
import { Session } from '../../noyau/session';
import { teteDuJoueur } from '../../noyau/pont';

/**
 * Le joueur connecté, en bas à droite, et le menu de son compte.
 *
 * ## La tête est une IMAGE, pas une icône
 *
 * Un rendu de tête par UUID, servi par `mc-heads.net` — c'est pourquoi
 * `img-src` le nomme dans le CSP, et c'est le seul hôte distant que la fenêtre
 * atteigne. `image-rendering: pixelated` : une texture de seize pixels agrandie
 * à quarante doit rester une texture, pas une aquarelle.
 *
 * Tant qu'elle n'est pas arrivée, le cube neutre du design system tient la
 * place. Pas d'initiales : elles feraient croire à un avatar alors que ce qui
 * va arriver est une tête de jeu.
 *
 * ## Ce qui n'est PAS montré
 *
 * L'UUID, l'adresse Microsoft, le jeton. Le design system l'interdit, et la
 * raison tient en une phrase : ce sont les trois choses qu'un joueur recopiera
 * dans un salon Discord si l'interface les met sous ses yeux.
 */
@Component({
  selector: 'app-joueur',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './joueur.html',
  styleUrl: './joueur.css',
})
export class Joueur {
  private readonly session = inject(Session);
  private readonly incidents = inject(Incidents);
  private readonly notifications = inject(Notifications);
  private readonly router = inject(Router);

  protected readonly compte = this.session.compte;
  protected readonly occupe = this.session.occupe;

  /** Le menu du compte est-il déplié ? */
  protected readonly menuOuvert = signal(false);

  protected readonly tete = computed(() => {
    const joueur = this.compte();
    return joueur ? teteDuJoueur(joueur.uuid) : null;
  });

  /** La tête a-t-elle échoué à charger ? On retombe alors sur le cube. */
  protected readonly teteAbsente = signal(false);

  protected readonly ChevronDown = ChevronDown;
  protected readonly LogOut = LogOut;
  protected readonly RefreshCw = RefreshCw;
  protected readonly User = User;

  protected basculerMenu(): void {
    this.menuOuvert.update((ouvert) => !ouvert);
  }

  protected fermerMenu(): void {
    this.menuOuvert.set(false);
  }

  /**
   * Redemande le statut à Rust.
   *
   * Utile quand une licence vient d'être achetée, ou quand le jeton a été
   * renouvelé en arrière-plan : sans ce geste, il faudrait relancer le launcher
   * pour que la fenêtre s'en aperçoive.
   */
  protected async rafraichir(): Promise<void> {
    this.fermerMenu();
    await this.incidents.pendant(async () => {
      await this.session.ouvrir();
      this.notifications.signaler('info', 'Session rafraîchie', this.compte()?.pseudo ?? null);
    });
  }

  protected async deconnecter(): Promise<void> {
    this.fermerMenu();
    await this.incidents.pendant(async () => {
      const pseudo = this.compte()?.pseudo ?? null;
      await this.session.deconnecter();
      this.notifications.signaler('info', 'Déconnecté', pseudo);
      // La garde de `/spawn` renverrait de toute façon, mais l'attendre
      // laisserait une page qui parle d'un compte qui n'existe plus le temps
      // d'un rendu. On y va nous-mêmes.
      await this.router.navigate(['/connexion']);
    });
  }
}
