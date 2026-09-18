#!/usr/bin/env node
/**
 * Le CSS produit porte-t-il encore le préfixe dont WebKitGTK a besoin ?
 *
 * ## Ce qui se casse, et pourquoi rien ne le dirait
 *
 * La WebKitGTK du launcher (2.52.5, `libwebkit2gtk-4.1.so.0.21.9`) n'expose
 * QUE `-webkit-backdrop-filter` : la forme non préfixée est absente de la
 * bibliothèque. Angular ne passe pas par autoprefixer — il traduit la
 * `browserslist` de package.json en cibles esbuild, et esbuild est le seul à
 * préfixer. Le seuil est exactement Safari 18 : au-dessus, il n'émet plus que
 * la forme moderne.
 *
 * Conséquence : remplacer `safari 17.4` par n'importe quelle cible plus
 * récente — ou par le défaut `> 1%` d'Angular — fait disparaître le préfixe,
 * et chaque `backdrop-blur-*` de l'interface cesse d'avoir le moindre effet
 * DANS LA FENÊTRE. Le build réussit, les tests passent, la CI est verte, et
 * le verre dépoli rend plat. Personne ne relie ça à une ligne de
 * `browserslist` modifiée trois semaines plus tôt.
 *
 * ## Le piège que ce script existe aussi pour fermer
 *
 * Le design system écrit lui-même les deux formes, ce qui rend ce contrôle
 * moins critique qu'il ne l'était du temps des utilitaires produits par un
 * outil. Il garde son sens pour le CSS que NOUS écrivons — les styles de
 * composant — où le préfixe n'est jamais écrit à la main : c'est esbuild qui
 * le pose, et il ne le fait qu'en dessous de Safari 18.
 *
 * D'où la vérification dans le CSS COMPILÉ et non dans les sources : un
 * préfixe écrit à la main quelque part ferait passer le contrôle — esbuild ne
 * retire pas un préfixe écrit à la main — tout en laissant sans préfixe tout
 * ce qui ne l'a pas.
 */

import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SORTIE = fileURLToPath(new URL('../dist/launcher/browser', import.meta.url));

let feuilles;
try {
  feuilles = readdirSync(SORTIE).filter((nom) => nom.endsWith('.css'));
} catch {
  console.error(`Aucune sortie de build sous ${SORTIE}. Lancer « pnpm build » d'abord.`);
  process.exit(1);
}

if (feuilles.length === 0) {
  console.error("Le build n'a produit aucune feuille de style.");
  process.exit(1);
}

const css = feuilles.map((nom) => readFileSync(join(SORTIE, nom), 'utf8')).join('\n');

const prefixe = css.includes('-webkit-backdrop-filter');
// La regex évite de compter le préfixe une seconde fois : on cherche la
// propriété précédée d'autre chose qu'un tiret.
const nonPrefixe = /(^|[^-])backdrop-filter\s*:/.test(css);

if (!nonPrefixe && !prefixe) {
  console.log(
    '  ~ aucun backdrop-filter dans le CSS produit — rien à vérifier tant\n' +
      "    qu'aucun composant n'emploie le verre dépoli.",
  );
  process.exit(0);
}

if (nonPrefixe && !prefixe) {
  console.error(
    '\n  ✗ backdrop-filter est émis SANS le préfixe -webkit-.\n' +
      '\n    La WebKitGTK du launcher ne connaît que la forme préfixée : tout le\n' +
      '    verre dépoli de la fenêtre va rendre plat, en production seulement.\n' +
      '\n    Cause quasi certaine : la « browserslist » de package.json a été\n' +
      "    modifiée. Elle doit rester sous Safari 18 — c'est le seuil auquel\n" +
      '    esbuild cesse de préfixer. Voir le commentaire de ce script.\n',
  );
  process.exit(1);
}

console.log('  ✓ -webkit-backdrop-filter présent dans le CSS produit');
