import { inject } from '@angular/core';
import { Router, type CanActivateFn, type Routes } from '@angular/router';

import { Session } from './noyau/session';

/**
 * Le joueur peut-il accéder aux pages du launcher ?
 *
 * ## Le prédicat est UNIQUE, et c'est tout le sujet
 *
 * Les deux gardes lisent `session.jouable()`, et rien d'autre. Deux prédicats
 * différents — « connecté » d'un côté, « jouable » de l'autre — feraient
 * rebondir sans fin un compte connecté SANS LICENCE : la garde des pages le
 * renverrait vers `/connexion`, celle de la connexion le trouverait connecté
 * et le renverrait vers les pages, et ainsi de suite jusqu'à ce que le routeur
 * abandonne.
 *
 * Ce cas n'est pas théorique : c'est celui d'un compte Microsoft valide qui
 * n'a jamais acheté Minecraft. Il est traité comme un ÉTAT de la page
 * Connexion, avec son propre message, et non comme une redirection.
 *
 * ## Pourquoi la garde attend
 *
 * `ouvrir()` interroge Rust, ce qui prend le temps de deux allers-retours
 * réseau. Le routeur ATTEND une garde qui rend une promesse et ne valide
 * aucune URL tant qu'elle pend : il n'y a donc aucun saut à empêcher, et c'est
 * pour cela qu'aucun `withDisabledInitialNavigation()` n'est posé.
 */
const jouable: CanActivateFn = async () => {
  const session = inject(Session);
  const router = inject(Router);

  if (!session.connue()) {
    await session.ouvrir();
  }
  return session.jouable() ? true : router.createUrlTree(['/connexion']);
};

/** L'inverse, sur EXACTEMENT le même prédicat. */
const pasEncoreJouable: CanActivateFn = async () => {
  const session = inject(Session);
  const router = inject(Router);

  if (!session.connue()) {
    await session.ouvrir();
  }
  return session.jouable() ? router.createUrlTree(['/spawn']) : true;
};

/**
 * Les routes.
 *
 * ## La donnée `bas`
 *
 * Elle dit ce que la barre du bas de la fenêtre porte : le bouton de jeu et le
 * badge joueur, le badge seul, ou rien — auquel cas la page occupe deux rangs
 * au lieu de trois. C'est une DONNÉE de route et non un test sur l'URL : une
 * chaîne comparée à `'/spawn'` se casse le jour où une route gagne un
 * paramètre, et le symptôme est une barre du bas vide que rien n'explique.
 *
 * ## Tout est paresseux, sans exception
 *
 * `loadComponent` sur chacune : le morceau d'une page n'est téléchargé qu'au
 * moment où on y va. Sur un launcher, cela se voit — la page de configuration
 * et celle des nouvelles ne sont ouvertes qu'une fois sur dix, et les charger
 * au démarrage retarderait l'écran que tout le monde regarde.
 *
 * ## `pathMatch: 'full'` sur la redirection vide
 *
 * Sans lui, Angular refuse la route avec NG04014 : une redirection depuis un
 * chemin vide sans `pathMatch` est ambiguë, puisque le chemin vide est un
 * préfixe de tout.
 */
export const ROUTES: Routes = [
  {
    path: 'connexion',
    canActivate: [pasEncoreJouable],
    // La connexion est une modale par-dessus la scène : pas de coque du tout,
    // donc pas de barre du bas. Voir `app.html`.
    data: { bas: 'aucune' },
    loadComponent: () => import('./connexion/connexion').then((m) => m.Connexion),
  },
  {
    path: 'spawn',
    canActivate: [jouable],
    data: { bas: 'jouer' },
    loadComponent: () => import('./spawn/spawn').then((m) => m.Spawn),
  },
  {
    path: 'nouvelles',
    canActivate: [jouable],
    // Le badge joueur reste, le bouton de jeu non : le design system garde la
    // barre du bas sur les Nouvelles, avec rien au centre.
    data: { bas: 'joueur' },
    loadComponent: () => import('./nouvelles/nouvelles').then((m) => m.PageNouvelles),
  },
  {
    path: 'configuration',
    canActivate: [jouable],
    // La Configuration LAISSE TOMBER la barre du bas, et le corps prend la
    // hauteur : c'est une règle explicite du design system, et c'est ce qui
    // donne à la liste de réglages de quoi défiler.
    data: { bas: 'aucune' },
    loadComponent: () => import('./configuration/configuration').then((m) => m.Configuration),
  },
  { path: '', pathMatch: 'full', redirectTo: 'spawn' },
  // Un chemin inconnu ne doit pas laisser une fenêtre vide : dans une
  // application de bureau, il n'y a pas de barre d'adresse pour s'en sortir.
  { path: '**', redirectTo: 'spawn' },
];
