/**
 * Dire une date de billet.
 *
 * ## Pourquoi pas `Intl.RelativeTimeFormat` seul
 *
 * « il y a 3 jours » se lit bien pour ce qui est récent et mal pour le reste :
 * « il y a 47 jours » demande un calcul mental que « 2 août » ne demande pas.
 * On bascule donc à une semaine, qui est à peu près l'horizon où l'on cesse de
 * compter.
 *
 * ## Une fonction pure, et testée
 *
 * Elle prend l'instant de référence en paramètre. Sans cela, il faudrait
 * truquer l'horloge pour l'éprouver, et les tests dépendraient du jour où on
 * les lance.
 */
export function direLaDate(iso: string, maintenant: Date = new Date()): string {
  const quand = new Date(iso);
  if (Number.isNaN(quand.getTime())) {
    // Rust valide déjà le format, mais un fil servi par un hôte modifié
    // pourrait passer autre chose. Afficher « Invalid Date » serait pire que
    // de ne rien dire.
    return '';
  }

  const secondes = Math.round((maintenant.getTime() - quand.getTime()) / 1000);
  const relatif = new Intl.RelativeTimeFormat('fr', { numeric: 'auto' });

  if (secondes < 60) {
    return "à l'instant";
  }
  if (secondes < 3600) {
    return relatif.format(-Math.round(secondes / 60), 'minute');
  }
  if (secondes < 86_400) {
    return relatif.format(-Math.round(secondes / 3600), 'hour');
  }
  if (secondes < 7 * 86_400) {
    return relatif.format(-Math.round(secondes / 86_400), 'day');
  }

  // Au-delà d'une semaine : une date, avec l'année seulement si ce n'est pas
  // celle en cours — « 2 août 2024 » quand c'est utile, « 2 août » sinon.
  const memeAnnee = quand.getUTCFullYear() === maintenant.getUTCFullYear();
  return new Intl.DateTimeFormat('fr', {
    day: 'numeric',
    month: 'long',
    year: memeAnnee ? undefined : 'numeric',
    timeZone: 'UTC',
  }).format(quand);
}
