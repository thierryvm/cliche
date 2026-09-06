#!/usr/bin/env node
// ONE list of icon paths, for the whole product - and this is what makes that
// sentence true of `veil.html` rather than only of the React half.
//
// WHY THIS SCRIPT EXISTS AT ALL
//   src/design/Glyph.tsx opens by stating the rule: "A `d` string is never
//   copied. Two icon sets start identical and stop being so at the first
//   correction, and nothing in the build would notice." Everything that draws
//   an icon imports from that file - everything except one document, and it
//   cannot: `veil.html` is a separate entry point that must not load React,
//   and `Glyph.tsx` is a React module. So the veil's three glyphs are written
//   as literal `d` attributes, and this script is the thing that would notice.
//
// WHAT IT CHECKS
//   Every `d="..."` in veil.html is one of the paths ICON publishes, and the
//   three the veil is supposed to draw - check, alert, close - are all present.
//   The second half matters as much as the first: a `d` deleted from the veil
//   would pass a membership test trivially.
//
// WHY A SCRIPT AND NOT A VITEST TEST
//   Same reason as its three neighbours: it reads the file tree, which needs
//   `node:fs`, which needs `@types/node` - a dependency this project does not
//   have and would not buy for one test.
//
// No dependency, on purpose, like check-version, check-contrast and
// check-strings.
//
// KNOWN BLIND SPOT, stated so nobody trusts this past its reach: it says
// nothing about WHICH icon sits where. A veil that drew the alert path for a
// success would pass. What it rules out is the case that actually happens - a
// path corrected in `Glyph.tsx` and left behind in `veil.html`, or the reverse.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const GLYPHS = join(root, 'src', 'design', 'Glyph.tsx');
const VEIL = join(root, 'veil.html');

/** The icons veil.html has to be able to draw, and what each one is for. */
const REQUIRED = {
  check: 'the success confirmation',
  alert: 'the failure that stays until it is dismissed',
  close: 'the button that dismisses it',
};

/**
 * Reads `name: 'M4 8V5…',` pairs out of the ICON table.
 *
 * Deliberately naive, like check-strings' reading of the catalogue: one pair
 * per line, no parser, no dependency. `selfCheck` puts it to a sample whose
 * answer is known, so a parser that quietly stops finding anything is caught
 * rather than mistaken for a clean tree.
 */
function readIcons(source) {
  const found = new Map();

  for (const line of source.split(/\r?\n/)) {
    const match = /^\s{2}([a-z][A-Za-z0-9]*):\s*'([^']*)',\s*$/.exec(line);
    if (match) found.set(match[1], match[2]);
  }

  return found;
}

/** Every `d="…"` attribute in an HTML file, with the line it sits on. */
function readPaths(source) {
  const found = [];
  const pattern = /\sd="([^"]*)"/g;

  for (const match of source.matchAll(pattern)) {
    found.push({
      value: match[1],
      line: source.slice(0, match.index).split('\n').length,
    });
  }

  return found;
}

/**
 * Proves both readers can still tell yes from no.
 *
 * Without it this script would report "OK" just as loudly with a table parser
 * that finds no icon and an attribute reader that finds no path - and a green
 * run would mean only that the instrument is broken.
 */
function selfCheck() {
  const sample = ['export const ICON = {', "  sample: 'M1 2h3',", '} as const;'].join('\n');
  const parsed = readIcons(sample);
  if (parsed.get('sample') !== 'M1 2h3') {
    throw new Error(`the ICON parser failed its own sample: ${JSON.stringify([...parsed])}`);
  }

  const markup = '<svg>\n  <path d="M1 2h3" />\n</svg>';
  const paths = readPaths(markup);
  if (paths.length !== 1 || paths[0].value !== 'M1 2h3' || paths[0].line !== 2) {
    throw new Error(`the path reader failed its own sample: ${JSON.stringify(paths)}`);
  }

  // `d` must not be found inside another attribute's name - `data-foo="…"`
  // is the shape that would do it, and veil.html has one.
  if (readPaths('<svg data-toast-glyph="success" />').length !== 0) {
    throw new Error('the path reader matched a data- attribute, so every report below is noise');
  }
}

selfCheck();

const icons = readIcons(readFileSync(GLYPHS, 'utf8'));
if (icons.size === 0) {
  console.error(`FAIL: no icon was read out of ${relative(root, GLYPHS)}.`);
  process.exit(1);
}

const veil = readFileSync(VEIL, 'utf8');
const drawn = readPaths(veil);
const failures = [];

// --- 1. Every path the veil draws is one the system publishes. ---------------
const published = new Map();
for (const [name, value] of icons) published.set(value, name);

for (const { value, line } of drawn) {
  if (!published.has(value)) {
    failures.push(
      `veil.html:${line}: d="${value}" is not one of the ${icons.size} paths in ` +
        `${relative(root, GLYPHS).replaceAll('\\', '/')}.\n` +
        '        Either the icon was corrected there and not here, or a shape was ' +
        'invented for this page alone.\n        There is one icon set in this product; ' +
        'copy the string from ICON, or change ICON.',
    );
  }
}

// --- 2. And the three it has to draw are all still there. --------------------
const values = new Set(drawn.map((path) => path.value));

for (const [name, purpose] of Object.entries(REQUIRED)) {
  const expected = icons.get(name);

  if (expected === undefined) {
    failures.push(
      `src/design/Glyph.tsx: ICON.${name} no longer exists, and veil.html draws it for ` +
        `${purpose}.`,
    );
    continue;
  }
  if (!values.has(expected)) {
    failures.push(
      `veil.html: no <path> carries ICON.${name}, which is ${purpose}.\n` +
        `        Expected d="${expected}".\n` +
        '        A .c-note keeps its word AND its glyph in the markup (PRD A4): the ' +
        'colour is the third cue, never the first.',
    );
  }
}

console.log(`  ${String(icons.size).padStart(3)} icons in src/design/Glyph.tsx`);
console.log(`  ${String(drawn.length).padStart(3)} paths drawn in veil.html`);

if (failures.length > 0) {
  console.error(`\nFAIL: ${failures.length} problem(s).\n`);
  for (const failure of failures) console.error(`  - ${failure}\n`);
  process.exit(1);
}

console.log('\nOK: the veil draws the system\'s icons, and all of the ones it needs');
