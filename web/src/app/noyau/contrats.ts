/**
 * Ce que Rust sérialise, écrit une fois et lu partout.
 *
 * Ces types n'ont AUCUN code : ils décrivent le contrat du pont. Les garder à
 * part des services est ce qui permet de les lire d'un trait quand on cherche
 * ce qu'une commande rend — et ce qui évite qu'un composant importe un service
 * entier pour un type.
 *
 * Chaque nom est écrit des deux côtés. Les renommer d'un seul laisse une
 * fenêtre qui n'affiche plus rien, sans qu'aucune compilation ne s'en plaigne :
 * c'est du JSON, et le JSON ne se plaint jamais.
 */

/** Sous quel nom le launcher se présente, figé à la compilation. */
export interface Marque {
  readonly nom: string;
  readonly sceau: string;
}

/** Le compte connecté. */
export interface Compte {
  readonly pseudo: string;
  readonly uuid: string;
  /**
   * Faux quand le compte n'a pas de licence Minecraft Java Edition. La
   * connexion réussit quand même — c'est un compte Microsoft valide — mais
   * aucun serveur en ligne ne l'acceptera.
   */
  readonly possedeLeJeu: boolean;
}

/** Ce que le joueur doit saisir chez Microsoft. */
export interface CodeAppareil {
  readonly code: string;
  readonly url: string;
  /** La même page, code prérempli. C'est celle qu'on ouvre. */
  readonly urlDirecte: string;
}

/** Les identifiants de phase, tels que `phase.rs` les sérialise. */
export type Phase =
  | 'connexion'
  | 'licence'
  | 'pack'
  | 'chargeur'
  | 'minecraft'
  | 'java'
  | 'neo-forge'
  | 'mods'
  | 'verrou'
  | 'pret'
  | 'lancement';

/** Une phase du chemin, avec de quoi la dessiner. */
export interface EtapeVue {
  readonly phase: Phase;
  readonly libelle: string;
  readonly rang: number;
}

/** L'état de l'installation, cinq fois par seconde. */
export interface Avancement {
  readonly phase: Phase;
  /** La phase est-elle un état atteint plutôt qu'un travail en cours ? */
  readonly achevee: boolean;
  readonly note: string | null;
  readonly fichier: string | null;
  readonly octets: number;
  readonly total: number;
  readonly fichiers: number;
  readonly fichiersTotal: number;
  /** Un téléchargement est-il réellement en cours ? */
  readonly actif: boolean;
  readonly debit: number;
  readonly restant: number | null;
}

/** Ce que le bouton doit dire. */
export type Action = 'installer' | 'jouer';

/** Ce qui sépare le posé du publié. */
export type Ecart = 'absent' | 'a-jour' | 'mise-a-jour' | 'reinstallation' | 'inconnu';

/** Ce que le disque et le pack publié disent, sans rien installer. */
export interface EtatDuPack {
  readonly action: Action;
  readonly ecart: Ecart;
  readonly horsLigne: boolean;
  readonly installe: boolean;
  readonly nom: string | null;
  readonly version: string | null;
  readonly java: number | null;
  readonly mods: number;
  readonly generation: number;
}

/**
 * Ce qu'un geste a laissé derrière lui.
 *
 * **Le même type pour installer et pour jouer** : les deux laissent les mêmes
 * traces — des mods introuvables, des écarts au verrou, une purge — et seul le
 * `verdict` diffère. Deux types jumeaux obligeraient l'écran à porter deux
 * chemins d'affichage pour dire la même chose.
 */
export interface CompteRendu {
  readonly verdict: string;
  /** Quelque chose a-t-il été posé ? */
  readonly rattrapee: boolean;
  readonly introuvables: readonly string[];
  readonly ecarts: readonly string[];
  readonly horsLigne: boolean;
  /** Ce que la purge a effacé, s'il y a eu purge. */
  readonly purge: readonly string[];
}

// --- Les nouvelles ---------------------------------------------------------

/**
 * Un fragment de texte d'un billet.
 *
 * Une union DISCRIMINÉE sur `type` : c'est ce que `#[serde(tag = "type")]`
 * produit côté Rust, et c'est ce qui permet au gabarit d'écrire un `@switch`
 * ordinaire. Sans le tag, serde rendrait `{"Gras": {...}}` et il faudrait
 * inspecter la première clé d'un objet.
 */
export type Inline =
  | { readonly type: 'texte'; readonly texte: string }
  | { readonly type: 'gras'; readonly texte: string }
  | { readonly type: 'italique'; readonly texte: string }
  | { readonly type: 'code'; readonly texte: string }
  | { readonly type: 'lien'; readonly texte: string; readonly href: string };

/** Un bloc de niveau supérieur d'un billet. */
export type Bloc =
  | { readonly type: 'paragraphe'; readonly contenu: readonly Inline[] }
  | { readonly type: 'titre'; readonly niveau: number; readonly contenu: readonly Inline[] }
  | { readonly type: 'liste'; readonly puces: readonly (readonly Inline[])[] }
  | { readonly type: 'separateur' };

export interface Billet {
  readonly id: string;
  readonly titre: string;
  /** RFC 3339 en UTC. Déjà validée par Rust. */
  readonly date: string;
  readonly epinglee: boolean;
  /** Un `data:` fabriqué par Rust, ou `null`. Jamais une URL distante. */
  readonly image: string | null;
  /**
   * Le corps, en arbre typé. **Jamais du HTML** : c'est la condition à
   * laquelle le CSP a été desserré, et la raison pour laquelle aucun
   * `innerHTML` n'existe dans ce dépôt.
   */
  readonly corps: readonly Bloc[];
}

export interface Fil {
  readonly billets: readonly Billet[];
  /** Le fil vient de la dernière copie connue et non du réseau. */
  readonly horsLigne: boolean;
  /** Les billets écartés, et pourquoi. */
  readonly ecartes: readonly string[];
}

// --- Les réglages ----------------------------------------------------------

export type ModeFenetre = 'fenetree' | 'maximisee' | 'plein-ecran';
export type Fond = 'spawn' | 'nether' | 'fin' | 'uni';

export interface ReglagesJeu {
  renderDistance: number;
  simulationDistance: number;
  maxFps: number;
  guiScale: number;
  vsync: boolean;
}

export interface ReglagesFenetre {
  mode: ModeFenetre;
  largeur: number;
  hauteur: number;
}

export interface ReglagesLanceur {
  memoireMo: number | null;
  reduireAuLancement: boolean;
}

export interface ReglagesApparence {
  fond: Fond;
  voile: number;
}

export interface Reglages {
  schema: number;
  jeu: ReglagesJeu;
  fenetre: ReglagesFenetre;
  lanceur: ReglagesLanceur;
  apparence: ReglagesApparence;
}

/** Ce que l'écran du joueur permet — zone UTILE, panneaux déduits. */
export interface Ecran {
  readonly largeur: number;
  readonly hauteur: number;
  readonly echelle: number;
}

/** Les dossiers qu'on propose d'ouvrir. Une liste fermée, côté Rust aussi. */
export type Dossier = 'donnees' | 'config' | 'journaux' | 'instance';
