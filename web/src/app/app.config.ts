import { ApplicationConfig, provideBrowserGlobalErrorListeners } from '@angular/core';
import { provideRouter, withComponentInputBinding, withHashLocation } from '@angular/router';

import { ROUTES } from './routes';

/**
 * Le strict nécessaire, et les deux options qui ne vont pas de soi.
 *
 * ## `withHashLocation()`
 *
 * Le protocole d'actifs de Tauri A un repli SPA — il sert `index.html` pour un
 * chemin qui n'existe pas. On ne s'y adosse pas pour autant : c'est un détail
 * d'implémentation du runtime, pas un contrat, et il ne vaut pas pour
 * `ng serve`, où l'on développe. Le fragment fonctionne partout, sans rien
 * demander à personne.
 *
 * ## Pas de `withDisabledInitialNavigation()`
 *
 * Il servirait à empêcher un saut d'écran pendant que les gardes interrogent
 * Rust. Or il n'y a pas de saut : une garde peut rendre une promesse, le
 * routeur l'attend, et aucune URL n'est validée tant qu'elle pend.
 *
 * Pire, le drapeau seul ne fait RIEN d'utile — il pose un jeton, et
 * `router.initialNavigation()` doit être appelé à la main ; l'oublier donne
 * une fenêtre bloquée sur son écran de démarrage, sans une erreur en console.
 * Et il supprime le seul recouvrement disponible : les morceaux paresseux ne
 * sont résolus qu'après les gardes, donc strictement après la poignée de main.
 *
 * ## `withComponentInputBinding()`
 *
 * Les paramètres d'URL arrivent en `input()` de composant, sans qu'aucune page
 * n'ait à s'abonner à `ActivatedRoute`. Rien ne l'emploie encore ; c'est une
 * ligne maintenant plutôt qu'une migration le jour où une page prendra un
 * identifiant de billet.
 */
export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    provideRouter(ROUTES, withHashLocation(), withComponentInputBinding()),
  ],
};
