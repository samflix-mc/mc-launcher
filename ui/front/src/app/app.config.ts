import { ApplicationConfig, provideBrowserGlobalErrorListeners } from '@angular/core';

/**
 * Le strict nécessaire.
 *
 * Pas de routeur : l'écran est unique, et il le restera tant que le launcher
 * n'a que deux boutons. Angular 22 est sans zone par défaut — les signaux du
 * composant suffisent à déclencher le rendu.
 */
export const appConfig: ApplicationConfig = {
  providers: [provideBrowserGlobalErrorListeners()],
};
