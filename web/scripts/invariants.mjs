#!/usr/bin/env node
/**
 * The front end's invariants, held by nothing else.
 *
 * These aren't style rules — ESLint and Prettier already handle those.
 * These are the conditions under which decisions were made, and which
 * would be lost silently if nobody checked them: breaking one fails no
 * test and reddens no linter, it loosens a guarantee.
 *
 * Written in Node and not in `grep`: the script runs in CI and on the
 * machine, and a `grep -r` doesn't behave the same way everywhere. Above
 * all, a grep line in package.json can't carry the reason — and a check
 * whose reason has been lost ends up removed because it's in the way.
 */

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('..', import.meta.url));
const SOURCE = join(ROOT, 'src');

/**
 * Each invariant: what's being looked for, where, and why it's forbidden.
 *
 * `active: false` marks the ones not yet held. Writing them before they can
 * be turned on is deliberate: it's the list of what's left to close, right
 * where it will be read again, rather than a line in a plan nobody reopens.
 */
const INVARIANTS = [
  {
    name: 'no innerHTML',
    active: true,
    // The TWO forms that actually write to the DOM, and only those:
    // property assignment, and Angular's property binding.
    //
    // The pattern was `\binnerHTML\b` at first, which triggered on the
    // comments explaining why none is used — a check that punishes its own
    // documentation ends up disabled, and with it go the cases it was
    // genuinely catching.
    pattern: /\.innerHTML\s*=|\[innerHTML\]|\binnerHTML\s*:/,
    extensions: ['.ts', '.html'],
    why: [
      "The launcher's CSP loosens `style-src` up to 'unsafe-inline' so that",
      'components can have their own styles. This loosening only holds',
      'because no markup comes from anywhere but the Angular compiler. A',
      'single innerHTML — on a news title, on an error message coming from',
      'Rust — and remote text would become DOM in a privileged origin where',
      '`invoke` is reachable. The news markdown is parsed in Rust into a',
      'typed tree for this exact reason.',
    ],
  },
  {
    name: 'no class selector in tests',
    // Enabled: the last five class assertions — `.etape`, `.jauge__glisseur`,
    // `.jauge__barre`, `.overlay`, `.app[data-flou]` — disappeared with the
    // interface redesign, and the templates now carry ninety `data-test`
    // attributes.
    //
    // It stayed at `false` for the whole time this code was being
    // replaced: a check that fails on what's currently being rewritten
    // gets disabled within two days, and never gets re-enabled.
    active: true,
    pattern: /querySelector(All)?\((['"`])\./,
    extensions: ['.ts'],
    why: [
      'Three families of attributes, and only one is a contract: `class` is',
      'read by the browser and the design system, `data-<state>` by styling',
      'and tests, `data-test` by tests alone. A test that targets a class',
      'breaks at the first layout change — and, worse, discourages changing it.',
    ],
  },
];

/** Every file under `src`, excluding generated directories. */
function files(root) {
  const found = [];
  for (const entry of readdirSync(root)) {
    const path = join(root, entry);
    if (statSync(path).isDirectory()) {
      found.push(...files(path));
    } else {
      found.push(path);
    }
  }
  return found;
}

const allFiles = files(SOURCE);
let failures = 0;

for (const invariant of INVARIANTS) {
  if (!invariant.active) {
    console.log(`  ~ ${invariant.name} — not enforced yet, check pending`);
    continue;
  }

  const violations = [];
  for (const path of allFiles) {
    if (!invariant.extensions.some((ext) => path.endsWith(ext))) {
      continue;
    }
    const lines = readFileSync(path, 'utf8').split('\n');
    lines.forEach((line, index) => {
      if (invariant.pattern.test(line)) {
        violations.push(`${relative(ROOT, path)}:${index + 1}`);
      }
    });
  }

  if (violations.length === 0) {
    console.log(`  ✓ ${invariant.name}`);
    continue;
  }

  failures += 1;
  console.error(`\n  ✗ ${invariant.name}`);
  for (const line of invariant.why) {
    console.error(`    ${line}`);
  }
  console.error('');
  for (const violation of violations) {
    console.error(`    ${violation}`);
  }
}

/**
 * Every `hm-*` class a template names must exist in a stylesheet.
 *
 * This one is a CROSS-CHECK and not a pattern, because the failure it
 * catches cannot be seen in a single file. A class the template writes and
 * no stylesheet declares is not an error anywhere: the HTML is valid, the
 * CSS is valid, the build is green, and the rule simply never applies. The
 * window then renders with one layout rule missing — and the symptom is a
 * black band, or a nav glued to the content, which nobody traces back to a
 * spelling.
 *
 * It is written after a real occurrence: translating the code base to
 * English renamed `.hm-window--system` to `.hm-window--system` in the
 * stylesheet and left the template spelling untouched. That class carries
 * `width: 100%; height: 100%`. Three screens shipped with a black band
 * below the content, and every test stayed green — the tests asserted the
 * template's class name against itself, never against the CSS.
 *
 * Only `hm-*` is checked: those come from the design system and are the
 * ones a rename can silently detach. Utility and component classes are
 * owned by the file that uses them.
 */
const HOOKS_WITHOUT_RULES = new Set([
  // Declared on the button so that a theme can reach it; only `--close`
  // needs a rule of its own, for its red hover.
  'hm-winctl__btn--min',
  'hm-winctl__btn--max',
]);

function declaredClasses() {
  const declared = new Set();
  for (const path of allFiles.filter((p) => p.endsWith('.css'))) {
    const text = readFileSync(path, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
    for (const [, name] of text.matchAll(/\.([a-zA-Z][\w-]*)/g)) {
      declared.add(name);
    }
  }
  return declared;
}

function usedClasses() {
  const used = [];
  for (const path of allFiles) {
    if (!path.endsWith('.html') && !path.endsWith('.ts')) continue;
    if (path.endsWith('.spec.ts')) continue;
    const text = readFileSync(path, 'utf8');
    const found = new Set();
    for (const [, group] of text.matchAll(/class="([^"]*)"/g)) {
      for (const name of group.split(/\s+/)) found.add(name);
    }
    // `[class.hm-window--maximized]="maximized()"` — a binding, invisible
    // to the plain `class="..."` scan, and exactly how one of them hid.
    for (const [, name] of text.matchAll(/\[class\.([\w-]+)\]/g)) found.add(name);
    for (const [, name] of text.matchAll(/['"`](hm-[\w-]+)['"`]/g)) found.add(name);
    for (const name of found) {
      // A trailing `--` is a concatenation base: `'hm-status--' + variant()`.
      if (name.startsWith('hm-') && !name.endsWith('--')) {
        used.push([name, relative(ROOT, path)]);
      }
    }
  }
  return used;
}

const declared = declaredClasses();
const orphans = usedClasses().filter(
  ([name]) => !declared.has(name) && !HOOKS_WITHOUT_RULES.has(name),
);

if (orphans.length === 0) {
  console.log('  \u2713 every hm-* class used has a rule');
} else {
  failures += 1;
  console.error('\n  \u2717 hm-* class used with no rule anywhere');
  console.error('    A template names a design-system class that no stylesheet');
  console.error('    declares. The rule never applies, and nothing reports it.');
  console.error('');
  for (const [name, where] of orphans) {
    console.error(`    ${where}: ${name}`);
  }
}

if (failures > 0) {
  console.error(`\n${failures} invariant(s) broken.`);
  process.exit(1);
}
