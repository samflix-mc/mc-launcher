/**
 * Les phrases de l'écran de démarrage.
 *
 * ## Pourquoi une liste et non un message d'état
 *
 * Sam a demandé une phrase configurable sous le chargeur, en laissant les deux
 * options ouvertes : des messages d'état réels — « configuration en cours » —
 * ou des phrases amusantes. Les deux sont possibles ici ; ce fichier porte les
 * secondes.
 *
 * Le motif du choix : l'amorce dure NEUF CENTS MILLISECONDES au plus. Un
 * message d'état qui changerait trois fois en moins d'une seconde ne se lit
 * pas — il clignote. Une phrase tenue tout du long se lit, et fait passer
 * l'attente pour une intention plutôt que pour une lenteur.
 *
 * ## Le tirage
 *
 * `Math.random()` est parfaitement adapté ici : personne ne dépend de la
 * phrase, et une répétition ne coûte rien. Le seul soin est qu'il soit appelé
 * UNE fois par ouverture et non à chaque rendu — sinon la phrase changerait à
 * chaque détection de changement, ce qui est le contraire de lisible.
 */
export const PHRASES: readonly string[] = [
  'Chargement des blocs…',
  'Réveil des villageois…',
  'Vérification des coffres…',
  'Alignement des redstones…',
  'Le creeper ne vous a pas vu.',
  'Compilation des mods, pas des excuses.',
  'Nourrissage des loups…',
  'Recherche du spawn perdu…',
  'Polissage de la pierre taillée…',
  'Négociation avec les Piglins…',
];

/** Une phrase, tirée une fois. */
export function unePhrase(): string {
  return PHRASES[Math.floor(Math.random() * PHRASES.length)];
}
