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

if (failures > 0) {
  console.error(`\n${failures} invariant(s) broken.`);
  process.exit(1);
}
