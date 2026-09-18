import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { CarteNouvelle } from '../nouvelles/carte/carte-nouvelle';
import * as format from '../noyau/format';
import { Incidents } from '../noyau/incidents';
import { Nouvelles } from '../noyau/nouvelles';
import { Pack } from '../noyau/pack';
import { Session } from '../noyau/session';
import { teteDuJoueur } from '../noyau/pont';

/**
 * L'écran principal — « Spawn ».
 *
 * Le nom vient de Sam : « accueil » ne disait rien d'un launcher Minecraft, et
 * « Spawn » est l'endroit où l'on arrive. Il est employé tel quel dans le menu,
 * dans les routes et ici, sans traduction intermédiaire.
 *
 * ## Le bouton unique
 *
 * Un seul, et il dit INSTALLER ou JOUER selon ce que le DISQUE impose. Le
 * cliquer déclenche toujours la même chose : vérifier le verrou publié, le
 * comparer à ce qui est posé, rattraper ce qui a bougé par différence, puis
 * lancer.
 *
 * L'état affiché a cinq valeurs et non deux — voir `Pack.bouton` : l'action
 * vient du disque, l'activité s'observe, et « on regarde encore » est un état
 * à part entière. Sans lui, un défaut « Installer » produirait un clignotement
 * INSTALLER → JOUER à chaque démarrage, sur le seul élément de l'écran qui
 * compte.
 */
@Component({
  selector: 'app-spawn',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [CarteNouvelle],
  templateUrl: './spawn.html',
  styleUrl: './spawn.css',
})
export class Spawn {
  private readonly pack = inject(Pack);
  private readonly incidents = inject(Incidents);
  private readonly nouvelles = inject(Nouvelles);

  protected readonly compte = inject(Session).compte;

  protected readonly etat = this.pack.etat;
  protected readonly bouton = this.pack.bouton;
  protected readonly etapes = this.pack.etapes;
  protected readonly avancement = this.pack.avancement;
  protected readonly progression = this.pack.progression;
  protected readonly etapeCourante = this.pack.etapeCourante;
  protected readonly partie = this.pack.derniereePartie;

  protected readonly derniereNouvelle = this.nouvelles.derniere;

  /** Le libellé du bouton, pour chacun des cinq états. */
  protected readonly libelle = computed(() => {
    switch (this.bouton()) {
      case 'inconnu':
        return 'Vérification…';
      case 'installer':
        return 'INSTALLER';
      case 'jouer':
        return 'JOUER';
      case 'occupe':
        return 'Installation…';
      case 'en-partie':
        return 'En jeu';
    }
  });

  /** Le bouton est-il cliquable ? */
  protected readonly actif = computed(() => {
    const etat = this.bouton();
    return etat === 'installer' || etat === 'jouer';
  });

  /** Ce que la comparaison a trouvé, en une phrase. */
  protected readonly resume = computed(() => {
    const etat = this.etat();
    if (!etat) {
      return null;
    }
    if (etat.horsLigne) {
      return 'Hors ligne : impossible de vérifier les mises à jour.';
    }
    switch (etat.ecart) {
      case 'absent':
        return 'Le pack n’est pas encore installé.';
      case 'a-jour':
        return 'Le pack est à jour.';
      case 'mise-a-jour':
        return 'Une mise à jour est disponible, elle sera appliquée au lancement.';
      case 'reinstallation':
        return 'Le pack demande une réinstallation complète — vos mondes et vos configurations sont conservés.';
      case 'inconnu':
        return null;
    }
  });

  /** Les mods introuvables de la dernière partie, s'il y en a. */
  protected readonly introuvables = computed(() => this.partie()?.introuvables ?? []);
  protected readonly ecarts = computed(() => this.partie()?.ecarts ?? []);
  protected readonly purge = computed(() => this.partie()?.purge ?? []);

  constructor() {
    void this.incidents.pendant(() => this.pack.ouvrir());
    // Le fil ne fait pas partie de ce que l'écran attend : une carte vide vaut
    // mieux qu'un écran qui attend le réseau pour montrer son bouton.
    void this.nouvelles.charger().catch(() => {});
  }

  protected async jouer(): Promise<void> {
    await this.incidents.pendant(() => this.pack.jouer());
  }

  protected tete(uuid: string): string {
    return teteDuJoueur(uuid);
  }

  protected initiales(pseudo: string): string {
    return format.initiales(pseudo);
  }

  protected teinte(identifiant: string): number {
    return format.teinte(identifiant);
  }

  protected octets(valeur: number): string {
    return format.octets(valeur);
  }

  protected debit(valeur: number): string {
    return format.debit(valeur);
  }

  protected duree(valeur: number): string {
    return format.duree(valeur);
  }

  protected pourcent(): number {
    return Math.round(this.progression());
  }
}
