#!/usr/bin/env node
// The interface speaks French from ONE file, src/strings.ts. This script is
// what makes that sentence true rather than aspirational, and it checks two
// different things:
//
//   1. WHERE THE WORDING COMES FROM. src/design/Showcase.tsx is the system on
//      one page - the artefact that gets looked at, argued over and signed off.
//      Every value in the catalogue must appear there, character for character.
//      A label reinvented in the catalogue is a label nobody ever saw on
//      screen.
//   2. THAT NOBODY RE-TYPES ONE. A catalogued label found anywhere else under
//      src/ is a second copy, and two copies of one sentence always drift. The
//      showcase is exempt by construction: it is deliberately autonomous, opens
//      in a plain browser tab, and imports nothing from the application.
//
// WHY THIS IS A SCRIPT AND NOT A VITEST TEST. Both checks read the file tree,
// which needs `node:fs`, which needs `@types/node` - a dependency this project
// does not have and would not buy for one test. The two checks already wired
// into `pnpm test` (check-version, check-contrast) are the same shape and the
// same reason.
//
// No dependency, on purpose, like its two neighbours.
//
// KNOWN BLIND SPOTS, stated so nobody trusts this past its reach:
//   - French text that is NOT in the catalogue is invisible to check 2. This
//     script proves there is one copy of each catalogued label; it cannot prove
//     a sentence was catalogued in the first place.
//   - The comment stripper below understands strings and comments, not regular
//     expression literals. A literal such as /'/ would open a string that never
//     closes and blind the rest of the file. No scanned file contains one
//     today; the day one does, this script under-reports rather than
//     over-reports, which is the wrong way round and worth remembering.

import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const CATALOGUE = join(root, 'src', 'strings.ts');
const SHOWCASE = join(root, 'src', 'design', 'Showcase.tsx');

/** Directories and files check 2 does not scan, and why, in one place. */
const EXEMPT = [
  // The showcase IS the wording. It is where the values are copied from.
  join(root, 'src', 'design'),
  // The catalogue itself, obviously.
  CATALOGUE,
];

/** A letter, a digit or an underscore - what a word may be made of. */
const WORDISH = /[\p{L}\p{N}_]/u;

/**
 * Decodes the one HTML entity that stands between JSX source and what a user
 * reads. JSX turns `&apos;` into a plain `'` before it reaches the screen, so
 * comparing the two without this would be comparing spellings, not text.
 */
function asRendered(source) {
  return source.replaceAll('&apos;', "'");
}

/**
 * Reads `key: 'value'` pairs out of the catalogue.
 *
 * Deliberately naive, like check-version.mjs's reading of Cargo.toml: one pair
 * per line, no parser, no dependency. `selfCheck` below runs it over a sample
 * whose answer is known, so a parser that quietly stops finding anything is
 * caught rather than mistaken for a clean tree.
 */
function readCatalogue(source) {
  const found = new Map();

  for (const line of source.split(/\r?\n/)) {
    const match = /^\s{2}([a-z][A-Za-z0-9]*):\s*(['"])((?:[^\\]|\\.)*?)\2,\s*$/.exec(line);
    if (match) found.set(match[1], match[3].replaceAll("\\'", "'"));
  }

  return found;
}

/**
 * Blanks out comments, keeping every newline so reported line numbers stay
 * true.
 *
 * Comments are stripped BEFORE any search because they are written in English
 * and mention the interface constantly: `// Capture is an` in src/veil/main.ts
 * would otherwise be reported as a stray copy of the help heading.
 */
function withoutComments(source) {
  let out = '';
  let mode = 'code';
  let index = 0;

  while (index < source.length) {
    const character = source[index];
    const next = source[index + 1];

    if (mode === 'code') {
      if (character === '/' && next === '/') {
        mode = 'line';
        index += 2;
        continue;
      }
      if (character === '/' && next === '*') {
        mode = 'block';
        index += 2;
        continue;
      }
      if (character === "'" || character === '"' || character === '`') mode = character;
      out += character;
      index += 1;
      continue;
    }

    if (mode === 'line') {
      if (character === '\n') {
        mode = 'code';
        out += character;
      }
      index += 1;
      continue;
    }

    if (mode === 'block') {
      if (character === '*' && next === '/') {
        mode = 'code';
        index += 2;
        continue;
      }
      if (character === '\n') out += character;
      index += 1;
      continue;
    }

    // Inside a string literal of some kind.
    if (character === '\\') {
      out += character + (next ?? '');
      index += 2;
      continue;
    }
    if (character === mode) mode = 'code';
    out += character;
    index += 1;
  }

  return out;
}

/**
 * Every offset at which `phrase` occurs as a whole phrase.
 *
 * The word boundaries are what keep `Capture` from matching inside
 * `releasePointerCapture`. They are checked by hand rather than with `\b`,
 * which is ASCII-only in JavaScript and would misjudge a value starting with
 * `à` or `É`.
 */
function occurrences(haystack, phrase) {
  const hits = [];
  let from = 0;

  for (;;) {
    const at = haystack.indexOf(phrase, from);
    if (at === -1) return hits;

    const before = at === 0 ? '' : haystack[at - 1];
    const after = haystack[at + phrase.length] ?? '';
    if (!WORDISH.test(before) && !WORDISH.test(after)) hits.push(at);

    from = at + 1;
  }
}

/** 1-based line number of an offset, for a message somebody can act on. */
function lineOf(source, offset) {
  return source.slice(0, offset).split('\n').length;
}

/** Every .ts/.tsx file under src/, minus the exempt ones and the tests. */
function scannedFiles(directory) {
  const files = [];

  for (const item of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, item.name);
    if (EXEMPT.includes(path)) continue;

    if (item.isDirectory()) {
      files.push(...scannedFiles(path));
      continue;
    }
    if (!/\.tsx?$/.test(item.name) || /\.test\.tsx?$/.test(item.name)) continue;

    files.push(path);
  }

  return files;
}

/**
 * Proves the three functions above can still tell yes from no.
 *
 * Without it, this whole script would report "OK" just as loudly with a parser
 * that finds no key, a stripper that blanks everything, or a search that
 * matches nothing - and a green run would mean only that the instrument is
 * broken.
 */
function selfCheck() {
  const sample = ["export const UI_STRINGS = {", "  sampleKey: 'Fermer la porte',", '} as const;'].join('\n');
  const parsed = readCatalogue(sample);
  if (parsed.get('sampleKey') !== 'Fermer la porte') {
    throw new Error(`the catalogue parser failed its own sample: ${JSON.stringify([...parsed])}`);
  }

  const stripped = withoutComments("// Fermer\nconst a = 'Fermer';\n/* Fermer */\n");
  if (occurrences(stripped, 'Fermer').length !== 1) {
    throw new Error(`the comment stripper failed its own sample: ${JSON.stringify(stripped)}`);
  }

  if (occurrences('const releasePointerCapture = 1;', 'Capture').length !== 0) {
    throw new Error('the search matched inside an identifier, so every report below is noise');
  }
  if (occurrences('<h3>Capture</h3>', 'Capture').length !== 1) {
    throw new Error('the search missed a phrase between two tags, so it would report nothing');
  }
  if (occurrences(asRendered("<span>Capturer tout l&apos;écran</span>"), "Capturer tout l'écran").length !== 1) {
    throw new Error('the entity decoder failed its own sample');
  }
}

selfCheck();

const catalogue = readCatalogue(readFileSync(CATALOGUE, 'utf8'));
if (catalogue.size === 0) {
  console.error(`FAIL: no label was read out of ${relative(root, CATALOGUE)}.`);
  process.exit(1);
}

const failures = [];

// --- 1. Every label is one the showcase already publishes. -------------------
const showcase = asRendered(readFileSync(SHOWCASE, 'utf8'));

for (const [key, value] of catalogue) {
  if (occurrences(showcase, value).length === 0) {
    failures.push(
      `src/strings.ts: ${key} = ${JSON.stringify(value)} appears nowhere in ` +
        'src/design/Showcase.tsx.\n' +
        '        The showcase is where the wording is decided and looked at. Change it ' +
        'there first,\n        or copy from it character for character - including the ' +
        'ellipsis and the accents.',
    );
  }
}

// --- 2. Nobody re-types a label the catalogue already holds. -----------------
const files = scannedFiles(join(root, 'src'));

for (const path of files) {
  const source = withoutComments(asRendered(readFileSync(path, 'utf8')));

  for (const [key, value] of catalogue) {
    for (const at of occurrences(source, value)) {
      failures.push(
        `${relative(root, path).replaceAll('\\', '/')}:${lineOf(source, at)}: ` +
          `${JSON.stringify(value)} is written here AND in src/strings.ts.\n` +
          `        Import it instead: UI_STRINGS.${key}`,
      );
    }
  }
}

console.log(`  ${String(catalogue.size).padStart(3)} labels in src/strings.ts`);
console.log(`  ${String(files.length).padStart(3)} files scanned for copies of them`);

if (failures.length > 0) {
  console.error(`\nFAIL: ${failures.length} problem(s).\n`);
  for (const failure of failures) console.error(`  - ${failure}\n`);
  process.exit(1);
}

console.log('\nOK: every label is the showcase\'s, and written in one place');
