import { Injectable } from '@angular/core';
import type { UnlistenFn } from '@tauri-apps/api/event';

import type {
  Avancement,
  CodeAppareil,
  Compte,
  Dossier,
  Ecran,
  EtapeVue,
  EtatDuPack,
  Fil,
  Marque,
  Partie,
  Reglages,
} from './contrats';
import { DANS_TAURI, appeler, ecouter, ouvrirHorsApplication } from './transport';

const EVENEMENT_CODE = 'auth://code';
const EVENEMENT_AVANCEMENT = 'cinematique://avancement';

/**
 * Ce que Rust émet vers la fenêtre PRINCIPALE quand la connexion a abouti.
 *
 * Elle a chargé son front avant que la session n'existe — pendant que le joueur
 * s'authentifiait dans une autre fenêtre — et son service de session porte donc
 * un compte nul. Sans ce signal, elle se montrerait sur une page qu'elle n'a
 * plus de raison d'afficher. Écrit des deux côtés : voir `fenetres.rs`.
 */
const EVENEMENT_SESSION = 'session-ouverte';

/**
 * Le SEUL endroit qui parle à Rust.
 *
 * Tout passe par ici pour une raison précise : hors de la fenêtre Tauri —
 * `ng serve` seul, une suite de tests — `invoke` n'existe pas. Aucun composant
 * n'a à le savoir ; ils lisent `disponible`, ou passent par un service de
 * `noyau/` qui s'en charge.
 *
 * Ce service ne contient AUCUNE règle. Il appelle et il rend. Tout ce qui
 * décide vit soit dans un service du noyau, soit — de préférence — en Rust,
 * où la mutation l'atteint.
 */
@Injectable({ providedIn: 'root' })
export class Pont {
  /**
   * Y a-t-il quelqu'un à qui parler ?
   *
   * Vrai dans la fenêtre Tauri, et vrai AUSSI dans un navigateur quand le
   * serveur de développement tourne — c'est ce qui permet de regarder
   * l'interface ailleurs que dans la fenêtre. Faux dans une suite de tests,
   * où il n'y a ni l'un ni l'autre.
   */
  readonly disponible = DANS_TAURI || serveurDeDeveloppement();

  /**
   * Sommes-nous dans la FENÊTRE, et pas seulement devant un backend ?
   *
   * La distinction est née d'un plantage : `disponible` voulait dire « dans
   * Tauri » jusqu'à ce que le serveur de développement existe, et il veut
   * maintenant dire « il y a quelqu'un à qui parler ». Or `getCurrentWindow()`
   * ne s'adresse pas à un backend : c'est une API de la fenêtre, et hors
   * d'elle, elle lève.
   *
   * Tout ce qui pilote la FENÊTRE — réduire, agrandir, fermer — lit donc
   * ceci ; tout ce qui demande des DONNÉES lit `disponible`.
   */
  readonly dansLaFenetre = DANS_TAURI;

  // --- Les fenêtres --------------------------------------------------------

  /**
   * Ouvre la fenêtre de connexion, et efface la principale.
   *
   * Appelée depuis la fenêtre principale, dès qu'elle sait que la session n'est
   * pas jouable. Hors de Tauri — dans un navigateur, devant le serveur de
   * développement — il n'y a qu'un onglet : la commande ne fait rien, et c'est
   * le routeur qui emmène vers la page.
   */
  ouvrirConnexion(): Promise<void> {
    return appeler<void>('ouvrir_connexion');
  }

  /**
   * La session est ouverte : la principale reprend la main, la connexion s'en
   * va.
   */
  connexionReussie(): Promise<void> {
    return appeler<void>('connexion_reussie');
  }

  /**
   * La fenêtre principale annonce qu'elle a de quoi s'afficher.
   *
   * Rust l'ATTEND avant d'échanger les fenêtres : la montrer plus tôt ferait
   * voir un écran qui se remplit pendant deux secondes, ce que la fenêtre de
   * connexion couvre en restant lisible à sa place.
   */
  principalePrete(): Promise<void> {
    return appeler<void>('principale_prete');
  }

  /** La fenêtre principale apprend qu'une session vient de s'ouvrir ailleurs. */
  surSessionOuverte(recevoir: () => void): Promise<UnlistenFn> {
    return ecouter<unknown>(EVENEMENT_SESSION, () => recevoir());
  }

  // --- Le journal ----------------------------------------------------------

  /**
   * Écrit une ligne dans le journal de Rust, sous l'étiquette de cette fenêtre.
   *
   * L'étiquette n'est pas envoyée : Rust la lit sur la fenêtre appelante. C'est
   * le seul point de la chaîne où l'on ne peut pas se tromper de fenêtre — et
   * se tromper de fenêtre est le défaut que ce journal sert à traquer.
   */
  journal(niveau: string, message: string): Promise<void> {
    return appeler<void>('journal', { niveau, message });
  }

  // --- Le démarrage --------------------------------------------------------

  /**
   * Dit à Rust que le front a fini de se rendre.
   *
   * C'est ce qui referme l'écran de démarrage — une seconde fenêtre, sans
   * script, ouverte pendant qu'Angular se charge — et qui montre la fenêtre
   * principale, jusque-là cachée.
   *
   * Rust ne peut pas le deviner : il sait quand une fenêtre EXISTE, pas quand
   * son contenu est peint. Le front est le seul à le savoir.
   */
  frontPret(): Promise<void> {
    return appeler<void>('front_pret');
  }

  // --- Identité du launcher ------------------------------------------------

  marque(): Promise<Marque> {
    return appeler<Marque>('marque');
  }

  chemin(): Promise<EtapeVue[]> {
    return appeler<EtapeVue[]>('chemin');
  }

  // --- Session -------------------------------------------------------------

  statut(): Promise<Compte | null> {
    return appeler<Compte | null>('statut');
  }

  /** Ouvre une session Microsoft. Ne rend la main qu'une fois le code validé. */
  connexion(): Promise<Compte> {
    return appeler<Compte>('connexion');
  }

  deconnexion(): Promise<void> {
    return appeler<void>('deconnexion');
  }

  /**
   * S'abonne au code d'appareil.
   *
   * Il arrive par événement et non en valeur de retour : `connexion()` attend
   * que le joueur ait autorisé, et le code doit être affiché pendant cette
   * attente.
   */
  surCodeAppareil(recevoir: (code: CodeAppareil) => void): Promise<UnlistenFn> {
    return ecouter<CodeAppareil>(EVENEMENT_CODE, recevoir);
  }

  // --- Le pack -------------------------------------------------------------

  /** Ce que le disque et le pack publié disent. Quelques dizaines de kio. */
  etatDuPack(): Promise<EtatDuPack> {
    return appeler<EtatDuPack>('etat_du_pack');
  }

  /**
   * LE bouton : vérifie, rattrape s'il le faut, puis lance la partie.
   *
   * Ne rend la main qu'à la fin de la partie. L'avancement arrive par
   * événement pendant tout ce temps.
   */
  jouer(): Promise<Partie> {
    return appeler<Partie>('jouer');
  }

  verifierLesFichiers(profond: boolean): Promise<string[]> {
    return appeler<string[]>('verifier_les_fichiers', { profond });
  }

  surAvancement(recevoir: (avancement: Avancement) => void): Promise<UnlistenFn> {
    return ecouter<Avancement>(EVENEMENT_AVANCEMENT, recevoir);
  }

  // --- Les nouvelles -------------------------------------------------------

  nouvelles(): Promise<Fil> {
    return appeler<Fil>('nouvelles');
  }

  // --- Les réglages --------------------------------------------------------

  reglages(): Promise<Reglages> {
    return appeler<Reglages>('reglages');
  }

  /** Rend ce qui a été ÉCRIT, et non ce qu'on a envoyé. Voir `mc-reglages`. */
  enregistrerReglages(reglages: Reglages): Promise<Reglages> {
    return appeler<Reglages>('enregistrer_reglages', { reglages });
  }

  ecran(): Promise<Ecran | null> {
    return appeler<Ecran | null>('ecran');
  }

  ouvrirDossier(quoi: Dossier): Promise<void> {
    return appeler<void>('ouvrir_dossier', { quoi });
  }

  // --- Le système ----------------------------------------------------------

  /**
   * Ouvre une URL dans le navigateur DU SYSTÈME.
   *
   * Jamais dans la fenêtre : celle-ci est une origine privilégiée où `invoke`
   * est joignable, et le greffon de navigation de Rust refuserait de toute
   * façon. C'est le chemin de sortie de tout lien d'un billet.
   */
  ouvrirPage(url: string): Promise<void> {
    return ouvrirHorsApplication(url);
  }
}

/**
 * La tête du joueur, rendue en trois dimensions par mc-heads.net.
 *
 * Un service tiers, et c'est un choix : l'UUID part chez lui. Il est public —
 * n'importe quel serveur où le joueur se connecte le connaît — et c'est le
 * prix d'un vrai rendu de skin plutôt qu'une pastille de couleur. Le CSP de
 * `tauri.conf.json` autorise ce domaine et lui seul.
 */
export function teteDuJoueur(uuid: string, taille = 96): string {
  return `https://mc-heads.net/head/${encodeURIComponent(uuid)}/${taille}`;
}

/**
 * Le message d'une erreur venue de Rust.
 *
 * `Erreur` se sérialise en chaîne ; ce qui remonte autrement vient du pont
 * lui-même, et l'on ne veut pas afficher « [object Object] » à un joueur.
 */
export function messageDErreur(cause: unknown): string {
  if (typeof cause === 'string') {
    return cause;
  }
  if (cause instanceof Error) {
    return cause.message;
  }
  try {
    return JSON.stringify(cause) ?? String(cause);
  } catch {
    return String(cause);
  }
}

/**
 * Le serveur de développement est-il censé répondre ?
 *
 * On ne peut pas le savoir sans l'interroger, et `disponible` est lu de façon
 * synchrone par toute l'interface. On se fie donc au contexte : un navigateur
 * qui sert le front depuis la boucle locale est un poste de développement, et
 * c'est le seul cas où le serveur existe.
 *
 * Dans une suite de tests — jsdom — `location.hostname` est `localhost` mais
 * il n'y a pas de serveur : les appels échoueront, et c'est voulu. Les tests
 * qui comptent sur l'absence de pont passent par les services de `noyau/`,
 * lesquels lisent `disponible` et se taisent. C'est pourquoi `jsdom` est
 * écarté explicitement.
 */
function serveurDeDeveloppement(): boolean {
  if (typeof window === 'undefined' || navigator.userAgent.includes('jsdom')) {
    return false;
  }
  return ['localhost', '127.0.0.1'].includes(window.location.hostname);
}
