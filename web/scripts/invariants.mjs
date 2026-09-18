#!/usr/bin/env node
/**
 * Les invariants du front que rien d'autre ne tient.
 *
 * Ce ne sont pas des règles de style — ESLint et Prettier s'en chargent. Ce
 * sont les conditions auxquelles des décisions ont été prises, et qui se
 * perdraient en silence si personne ne les vérifiait : leur violation ne
 * casse aucun test et ne rougit aucun linter, elle desserre une garantie.
 *
 * Écrit en Node et non en `grep` : le script tourne en CI et sur le poste, et
 * un `grep -r` n'a pas le même comportement partout. Surtout, un grep en
 * ligne de package.json ne peut pas porter la raison — et un contrôle dont on
 * a perdu la raison finit par être retiré parce qu'il gêne.
 */

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const RACINE = fileURLToPath(new URL('..', import.meta.url));
const SOURCE = join(RACINE, 'src');

/**
 * Chaque invariant : ce qu'on cherche, où, et pourquoi c'est interdit.
 *
 * `actif: false` marque ceux qui ne sont pas encore tenus. Les écrire avant
 * de pouvoir les activer est délibéré : c'est la liste de ce qui reste à
 * fermer, à l'endroit où on la relira, plutôt qu'une ligne dans un plan que
 * personne ne rouvre.
 */
const INVARIANTS = [
  {
    nom: 'aucun innerHTML',
    actif: true,
    motif: /\binnerHTML\b/,
    extensions: ['.ts', '.html'],
    pourquoi: [
      "Le CSP du launcher desserre `style-src` jusqu'à 'unsafe-inline' pour",
      'que les composants aient leurs propres styles. Ce desserrage ne tient',
      "que parce qu'aucun balisage ne vient d'ailleurs que du compilateur",
      'Angular. Un seul innerHTML — sur un titre de news, sur un message',
      "d'erreur venu de Rust — et du texte distant deviendrait du DOM dans",
      'une origine privilégiée où `invoke` est joignable. Le markdown des',
      'nouvelles est analysé en Rust vers un arbre typé pour cette raison.',
    ],
  },
  {
    nom: 'aucun sélecteur de classe dans les tests',
    // À activer à la fin de E9, quand les cinq dernières assertions de classe
    // auront leur `data-test` : `.etape`, `.jauge__glisseur`, `.jauge__barre`,
    // `.overlay`, `.app[data-flou]`. L'activer avant ferait échouer le
    // contrôle sur du code qu'on est en train de remplacer.
    actif: false,
    motif: /querySelector(All)?\((['"`])\./,
    extensions: ['.ts'],
    pourquoi: [
      "Trois familles d'attributs, et une seule est un contrat : `class` est",
      'lue par le navigateur seul, `data-<état>` par Tailwind et les tests,',
      '`data-test` par les tests seuls. Un test qui vise une classe se casse',
      'au premier changement de mise en forme — et, pire, il décourage de la',
      'changer.',
    ],
  },
];

/** Tous les fichiers sous `src`, sans les répertoires générés. */
function fichiers(racine) {
  const trouves = [];
  for (const entree of readdirSync(racine)) {
    const chemin = join(racine, entree);
    if (statSync(chemin).isDirectory()) {
      trouves.push(...fichiers(chemin));
    } else {
      trouves.push(chemin);
    }
  }
  return trouves;
}

const tous = fichiers(SOURCE);
let echecs = 0;

for (const invariant of INVARIANTS) {
  if (!invariant.actif) {
    console.log(`  ~ ${invariant.nom} — pas encore tenu, contrôle en attente`);
    continue;
  }

  const fautifs = [];
  for (const chemin of tous) {
    if (!invariant.extensions.some((ext) => chemin.endsWith(ext))) {
      continue;
    }
    const lignes = readFileSync(chemin, 'utf8').split('\n');
    lignes.forEach((ligne, index) => {
      if (invariant.motif.test(ligne)) {
        fautifs.push(`${relative(RACINE, chemin)}:${index + 1}`);
      }
    });
  }

  if (fautifs.length === 0) {
    console.log(`  ✓ ${invariant.nom}`);
    continue;
  }

  echecs += 1;
  console.error(`\n  ✗ ${invariant.nom}`);
  for (const ligne of invariant.pourquoi) {
    console.error(`    ${ligne}`);
  }
  console.error('');
  for (const fautif of fautifs) {
    console.error(`    ${fautif}`);
  }
}

if (echecs > 0) {
  console.error(`\n${echecs} invariant(s) rompu(s).`);
  process.exit(1);
}
