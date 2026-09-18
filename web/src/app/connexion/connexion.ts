import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';

import { Incidents } from '../noyau/incidents';
import { Pont } from '../noyau/pont';
import { Session } from '../noyau/session';

/**
 * Ouvrir une session Microsoft.
 *
 * ## « Connecté mais sans licence » est un ÉTAT, pas une redirection
 *
 * Un compte Microsoft valide qui n'a jamais acheté Minecraft passe la
 * connexion et échoue au jeu. Les deux gardes du routeur lisent le MÊME
 * prédicat — `jouable()` — justement pour que ce cas ne rebondisse pas entre
 * elles : il reste ici, et l'écran le dit.
 *
 * Le dire coûte trois lignes ; ne pas le dire coûte huit cents mégaoctets
 * téléchargés pour apprendre ensuite qu'aucun serveur n'acceptera le compte.
 */
@Component({
  selector: 'app-connexion',
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './connexion.html',
  styleUrl: './connexion.css',
})
export class Connexion {
  private readonly session = inject(Session);
  private readonly incidents = inject(Incidents);
  private readonly pont = inject(Pont);
  private readonly router = inject(Router);

  protected readonly compte = this.session.compte;
  protected readonly code = this.session.code;
  protected readonly occupe = this.session.occupe;
  protected readonly sansLicence = this.session.sansLicence;

  protected async connecter(): Promise<void> {
    await this.incidents.pendant(async () => {
      await this.session.connecter();
      if (this.session.jouable()) {
        await this.router.navigate(['/spawn']);
      }
    });
  }

  protected async deconnecter(): Promise<void> {
    await this.incidents.pendant(() => this.session.deconnecter());
  }

  /**
   * Rouvre la page Microsoft.
   *
   * Rust l'a déjà tentée au moment où le code est arrivé ; ceci est le
   * recours, pour le cas où aucun navigateur n'était réglé par défaut.
   */
  protected async ouvrirLaPage(url: string): Promise<void> {
    await this.incidents.pendant(() => this.pont.ouvrirPage(url));
  }
}
