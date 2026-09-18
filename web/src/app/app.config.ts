import { ApplicationConfig, provideBrowserGlobalErrorListeners } from '@angular/core';
import { provideRouter, withComponentInputBinding } from '@angular/router';

import { ROUTES } from './routes';

/**
 * Le strict nécessaire, et les options qui ne vont pas de soi.
 *
 * ## Pas de `withHashLocation()` — des chemins réels
 *
 * Les routes sont `/spawn`, `/nouvelles`, `/configuration`. Il y avait un
 * fragment — `#/spawn` — dont le motif était la prudence : ne rien devoir au
 * repli SPA du protocole d'actifs de Tauri, qu'on tenait pour un détail
 * d'implémentation plutôt que pour un contrat.
 *
 * Il coûtait plus qu'il ne protégeait. Le dièse appartient au routeur dès qu'il
 * est en jeu, si bien qu'aucune ancre ne peut plus servir à autre chose dans la
 * page : le rail de la Configuration en a fait les frais — cliquer sur
 * `#reglages-video` écrivait une URL que le routeur essayait de résoudre comme
 * une route, et la navigation repartait vers `/spawn`.
 *
 * Le repli, lui, est bien là, et vérifié dans les sources de la version
 * épinglée : `tauri-2.11.5/src/manager/mod.rs` enchaîne quatre tentatives pour
 * un actif introuvable — `<chemin>.html`, `<chemin>/index.html`, puis
 * `index.html`. Et `ng serve` fait la même chose depuis toujours. Les deux
 * environnements où ce launcher tourne servent donc `index.html` pour une route
 * inconnue.
 *
 * Ce qu'il faut savoir si la fenêtre s'ouvrait blanche un jour : c'est cette
 * quatrième tentative qu'il faudrait vérifier, et `<base href="/">` dans
 * `index.html`, sans lequel les actifs se résoudraient contre `/spawn/`.
 *
 * Le greffon de navigation n'y change rien : son prédicat porte sur l'ORIGINE,
 * jamais sur le chemin.
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
    provideRouter(ROUTES, withComponentInputBinding()),
  ],
};
