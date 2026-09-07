#!/usr/bin/env node
// THE `hidden` ATTRIBUTE HAS TO ACTUALLY HIDE.
//
// WHY THIS SCRIPT EXISTS
//   `veil.html` parses its confirmation with `hidden` and shows it only once a
//   selection has been judged. On the installed v0.1.0, met on 7 September
//   2026, it was painted from the moment the veil appeared: an EMPTY
//   416 x 53 px box at the foot of the screen with a faint cross in it,
//   straddling the taskbar, over every capture. Thierry described it twice - a
//   black rectangle during a selection (dark theme) and a white one after.
//
//   Nothing in the veil's markup or script was wrong. `[hidden] { display:
//   none }` lives in the BROWSER's stylesheet, and every author declaration
//   beats a UA declaration whatever their specificity - that is the cascade's
//   origin order, not a specificity accident. `src/design/components.css`
//   declares `display: flex` on `.c-toast-region` and `display: inline-flex`
//   on `.c-btn`. The toast region wore the first, its dismiss button the
//   second, and both were rendered while their markup said they were not
//   there.
//
//   Two files, each correct read on its own, wrong together. No test in this
//   repository could have failed, and no review of either file would have
//   caught it: the collision is only visible holding the markup and the
//   stylesheet side by side. That is what this script does.
//
// WHAT IT CHECKS, and the third part is what keeps the first two honest
//   1. components.css carries a rule that enforces the attribute against its
//      own display declarations;
//   2. every element of veil.html wearing `hidden` is covered by it - and when
//      it is not, they are NAMED, or the next reader restarts the 7 September
//      investigation from a screenshot;
//   3. the collision is STILL REAL: `.c-toast-region` and `.c-btn` do still
//      declare `display`, and those two elements do still wear `hidden`.
//      Without (3) this gate goes green the day someone drops `display` from
//      `.c-btn` for an unrelated reason, and then guards nothing while looking
//      exactly as green as it does now.
//
// WHY A SCRIPT AND NOT A VITEST TEST
//   It was tried as one first, on 7 September 2026, and it cannot work:
//   Vitest's `test.css` is false by default, so `import css from '….css?raw'`
//   arrives as an EMPTY STRING - measured, 0 characters against 31 333 for the
//   same import of veil.html. Reading from disk instead needs `node:fs`, hence
//   `@types/node`, which is the dependency check-icons.mjs's own header refuses
//   to buy for one test. So: the same shape as its four neighbours.
//
// No dependency, on purpose, like check-version, check-contrast, check-strings
// and check-icons.
//
// KNOWN BLIND SPOT, stated so nobody trusts this past its reach: only selectors
// made of a SINGLE bare class are collected. `.c-note .c-btn__glyph { display:
// … }` sets display too and is not seen, because knowing whether it matches
// needs the DOM this script does not build. That can only make the gate MISS a
// collision, never invent one - and the rule it guards is universal, so a
// missed collision is covered anyway. The alternative, guessing at ancestry,
// would let this file claim something it cannot check.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const repoFile = (relative) =>
  readFileSync(fileURLToPath(new URL(`../${relative}`, import.meta.url)), 'utf8');

/**
 * Every element wearing a bare `hidden` attribute, with what a stylesheet could
 * select it by.
 *
 * A REGULAR EXPRESSION AND NOT A PARSER: a DOM parser is a dependency, and the
 * shape being looked for - an attribute inside a start tag - is within what a
 * regex answers honestly.
 *
 * `(^|\s)hidden(\s|=|>|\/|$)` and not `hidden`: without the leading boundary
 * `aria-hidden="true"` matches, and veil.html carries six of those. The
 * instrument check at the bottom holds that boundary.
 */
export const hiddenElementsOf = (html) => {
  const found = [];

  for (const tag of html.matchAll(/<([a-zA-Z][a-zA-Z0-9-]*)((?:"[^"]*"|'[^']*'|[^>"'])*)>/g)) {
    const [, name, attributes = ''] = tag;
    if (!/(^|\s)hidden(\s|=|>|\/|$)/.test(attributes)) continue;

    const id = /(^|\s)id\s*=\s*"([^"]*)"/.exec(attributes);
    const className = /(^|\s)class\s*=\s*"([^"]*)"/.exec(attributes);

    found.push({
      tag: name,
      id: id?.[2] ?? null,
      classes: (className?.[2] ?? '').split(/\s+/).filter((one) => one !== ''),
    });
  }

  return found;
};

/** A stylesheet with its comments gone, so a comment cannot be read as code. */
export const withoutComments = (css) => css.replace(/\/\*[\s\S]*?\*\//g, '');

/**
 * The class names a stylesheet declares `display` on, through a selector made
 * of that class alone.
 *
 * Comments are stripped FIRST, and that is not tidiness: the comment that
 * documents this very fix in components.css contains the words `display: none`
 * and the class names `.c-btn` and `.c-toast-region`. Reading it as code would
 * make this gate agree with itself.
 */
export const classesDeclaringDisplay = (css) => {
  const classes = new Set();

  for (const rule of withoutComments(css).matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const [, selector = '', body = ''] = rule;
    if (!/(^|[;{\s])display\s*:/.test(body)) continue;

    for (const one of selector.split(',')) {
      const bare = /^\s*\.([A-Za-z0-9_-]+)\s*$/.exec(one);
      if (bare) classes.add(bare[1]);
    }
  }

  return classes;
};

/**
 * Whether the stylesheet states that a hidden element is not laid out, in terms
 * no other rule of its own can outvote.
 *
 * `!important` is REQUIRED rather than merely accepted: a plain
 * `[hidden] { display: none }` repairs today and loses to the next rule written
 * under it, which is the same silent failure in a new costume.
 */
export const enforcesHiddenAttribute = (css) =>
  /\[hidden\][^{}]*\{[^{}]*display\s*:\s*none\s*!important/.test(withoutComments(css));

// ---------------------------------------------------------------------------
// THE INSTRUMENT, CHECKED BEFORE IT IS TRUSTED.
//
// A parser that silently returns nothing turns this whole gate green. These
// four cases are the ones that would do it, and they run every time rather than
// living in a test file the gate does not import.
// ---------------------------------------------------------------------------

const instrument = [];

{
  const sample = `
    <div class="a b" id="one" hidden></div>
    <span aria-hidden="true" class="c"></span>
    <p data-hidden="yes" class="d"></p>
    <em class="e" hidden="until-found"></em>
    <i class="hidden-looking"></i>
  `;
  const seen = hiddenElementsOf(sample).map((one) => `${one.tag}#${one.id ?? '-'}`);
  if (seen.join(',') !== 'div#one,em#-') {
    instrument.push(
      `hiddenElementsOf is broken: expected "div#one,em#-", got "${seen.join(',')}". ` +
        'It must ignore aria-hidden, data-hidden and a class that merely contains the word.',
    );
  }
}

{
  const sample = `
    .real { display: flex; }
    .also, .too { color: red; display: grid }
    .not { color: blue; }
    .descendant .child { display: block; }
    /* .commented { display: flex; } */
  `;
  const seen = [...classesDeclaringDisplay(sample)].sort().join(',');
  if (seen !== 'also,real,too') {
    instrument.push(
      `classesDeclaringDisplay is broken: expected "also,real,too", got "${seen}". ` +
        'A rule inside a comment must not count, and a bare class rule must.',
    );
  }
}

for (const [css, expected] of [
  ['[hidden] { display: none !important; }', true],
  ['[hidden] { display: none; }', false],
  ['.c-btn { display: none !important; }', false],
  ['/* [hidden] { display: none !important } */', false],
]) {
  if (enforcesHiddenAttribute(css) !== expected) {
    instrument.push(`enforcesHiddenAttribute is broken on: ${css}`);
  }
}

if (instrument.length > 0) {
  console.error('\nFAIL: this gate cannot be trusted - its own parser is wrong.\n');
  for (const problem of instrument) console.error(`  - ${problem}\n`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// THE REAL FILES.
// ---------------------------------------------------------------------------

const VEIL_HTML = repoFile('veil.html');
const COMPONENTS_CSS = repoFile('src/design/components.css');

const declaring = classesDeclaringDisplay(COMPONENTS_CSS);
const wearingHidden = hiddenElementsOf(VEIL_HTML);
const exposed = wearingHidden.filter((element) =>
  element.classes.some((one) => declaring.has(one)),
);

const failures = [];

if (!enforcesHiddenAttribute(COMPONENTS_CSS)) {
  failures.push(
    'src/design/components.css carries no rule making [hidden] win over its own display\n' +
      '        declarations. These elements of veil.html are therefore RENDERED while their\n' +
      '        markup says they are not there:\n' +
      exposed
        .map(
          (one) =>
            `          ${one.tag}#${one.id ?? '(no id)'} .${one.classes.join('.')}`,
        )
        .join('\n') +
      '\n        That is the empty 416 x 53 px box the veil showed on 7 September 2026.',
  );
}

// (3): the guard must not be guarding a collision that has quietly gone away.
for (const witness of ['c-toast-region', 'c-btn']) {
  if (!declaring.has(witness)) {
    failures.push(
      `.${witness} no longer declares \`display\` in components.css. The [hidden] rule may ` +
        'now be decorative.\n        Do not delete either: work out which rule lost its ' +
        '`display`, and whether this gate still watches anything.',
    );
  }
}
for (const witness of ['toast-region', 'toast-dismiss']) {
  if (!wearingHidden.some((one) => one.id === witness)) {
    failures.push(
      `veil.html: #${witness} no longer wears \`hidden\`. This gate was written around it; ` +
        'check it is still shown on demand rather than always.',
    );
  }
}

console.log(`  ${String(wearingHidden.length).padStart(3)} elements wear \`hidden\` in veil.html`);
console.log(`  ${String(declaring.size).padStart(3)} bare classes declare \`display\` in components.css`);
console.log(`  ${String(exposed.length).padStart(3)} of those elements need the guard to be hidden at all`);

if (failures.length > 0) {
  console.error(`\nFAIL: ${failures.length} problem(s).\n`);
  for (const failure of failures) console.error(`  - ${failure}\n`);
  process.exit(1);
}

console.log('\nOK: every hidden element of the veil is hidden, and the guard still guards');
