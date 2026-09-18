import { Injectable, inject } from '@angular/core';

import { Pont } from './pont';

/** Les niveaux que Rust sait relire. Voir `crates/mc-app/src/journal.rs`. */
export type Niveau = 'error' | 'warn' | 'info' | 'debug' | 'trace';

/**
 * Ce que le front raconte au journal de Rust.
 *
 * ## Pourquoi pas `console.log`
 *
 * La fenêtre n'a pas d'inspecteur en production : ce qu'on y écrit part dans le
 * vide, et c'est en production que se manifestent les défauts qu'on cherche.
 * Les lignes partent donc à Rust, qui les écrit dans SON journal — même
 * fichier, même horloge, même ordre que les siennes.
 *
 * C'est la seule façon de lire une séquence qui traverse deux fenêtres et un
 * processus. Recoller deux journaux à la main suppose de savoir dans quel ordre
 * les choses se sont produites, ce qui est justement la question posée.
 *
 * ## Il ne fait JAMAIS échouer l'appelant
 *
 * Journaliser est une opération de diagnostic : une ligne perdue coûte une
 * ligne, une exception levée depuis un `finally` de diagnostic coûte le geste
 * qu'on était en train d'observer. Tout est avalé.
 */
@Injectable({ providedIn: 'root' })
export class Journal {
  private readonly pont = inject(Pont);

  /** Un jalon de la séquence : ce que la fenêtre vient de faire. */
  etape(message: string): void {
    this.ecrire('info', message);
  }

  /** Un détail qu'on ne lit que lorsqu'on cherche. */
  detail(message: string): void {
    this.ecrire('debug', message);
  }

  /** Quelque chose d'inattendu, qui n'empêche pas de continuer. */
  souci(message: string): void {
    this.ecrire('warn', message);
  }

  private ecrire(niveau: Niveau, message: string): void {
    // Aussi en console : devant le serveur de développement, c'est là qu'on
    // regarde, et le double n'a aucun coût.
    console.info(`[${niveau}] ${message}`);
    void this.pont.journal(niveau, message).catch(() => {});
  }
}
