/**
 * The two HTML documents this product ships, held against what they claim.
 *
 * # Why a test can reach these at all
 *
 * `?raw` hands a file's source to the runner as a string. No DOM is built and
 * none is needed: what is checked here is the SOURCE, exactly as
 * `scripts/check-hidden.mjs` checks it - which is also where the technique and
 * its limits come from. A regular expression over a start tag is honest; a
 * regular expression pretending to be a parser is not.
 *
 * # What is checked, and the defect behind each
 *
 *   1. THE LANGUAGE. Both documents declared `lang="en"` until 7 September
 *      2026, while the veil already showed « copié » and « Échec » and the
 *      window showed « Capturer », « Aide » and « Réglages ». A wrong `lang` is
 *      not cosmetic: it is what a screen reader picks a voice from, and it is
 *      the attribute a previous run cited to refuse writing « Échec » - the
 *      attribute was the thing that was wrong.
 *   2. THE KEYBOARD LINE IS NOT WRITTEN IN THE MARKUP. It was, in English -
 *      « Enter or double-click copies · Esc cancels » - the one visible
 *      sentence of this product that never passed through `src/strings.ts`.
 *      Thierry met it on the installed v0.1.0 and read it as a rendering
 *      artefact.
 *   3. AND SOMETHING STILL FILLS IT. An empty element that nothing writes into
 *      is the 7 September defect in a new costume: veil.html already shipped an
 *      empty 416 x 53 px box at the foot of every capture. The plate is written
 *      once during the preheat, like the dismiss button's `aria-label` beside
 *      it, and this holds that line in place.
 *
 * # WHAT IS OUT OF REACH FROM HERE, and it is half of the plate
 *
 * `.c-veil-hint` itself lives in `src/design/components.css`, and this file
 * CANNOT read it: `import css from '….css?raw'` arrives as an empty string
 * under Vitest, because `test.css` is false by default. Re-measured on
 * 7 September 2026 while writing this file - 0 characters, against 31 333 for
 * the same import of `veil.html` - which is the same figure
 * `scripts/check-hidden.mjs` reports and the reason that gate is a script. So
 * the cap, the centring and the four forbidden properties are gated HERE only
 * for the veil's own stylesheet; for the component layer they are stated in
 * the rule's own comment and looked at in the showcase.
 */

import { describe, expect, it } from 'vitest';

import indexHtml from '../index.html?raw';
import veilHtml from '../veil.html?raw';
import veilMain from './veil/main.ts?raw';
import { UI_STRINGS } from './strings';

/** The veil's own stylesheet, comments stripped so one cannot be read as code. */
function veilStylesheet(): string {
  const style = /<style>([\s\S]*?)<\/style>/.exec(veilHtml);
  if (style === null) {
    throw new Error('veil.html no longer carries an inline <style>: this test reads nothing');
  }
  return (style[1] ?? '').replace(/\/\*[\s\S]*?\*\//g, '');
}

/** The body of one rule of that stylesheet, by selector. */
function veilRule(selector: string): string {
  const rule = new RegExp(`${selector}\\s*\\{([^{}]*)\\}`).exec(veilStylesheet());
  if (rule === null) {
    throw new Error(`veil.html declares no \`${selector}\` rule: this test reads nothing`);
  }
  return rule[1] ?? '';
}

describe('the documents', () => {
  it('declare the language they are actually written in', () => {
    expect(indexHtml).toMatch(/<html\s[^>]*lang="fr"/);
    expect(veilHtml).toMatch(/<html\s[^>]*lang="fr"/);
  });
});

describe("the veil's keyboard line", () => {
  it('carries no wording in the markup', () => {
    const plate = /<p id="hint"[^>]*>([\s\S]*?)<\/p>/.exec(veilHtml);

    expect(plate).not.toBeNull();
    expect(plate?.[1]?.trim()).toBe('');
  });

  it('is dressed by the system rather than by this document', () => {
    // The plate is a component now, so the showcase can publish it and the
    // failure it repaired - a black band from one edge of the screen to the
    // other - cannot come back unseen in a file nobody looks at.
    expect(veilHtml).toMatch(/<p id="hint"[^>]*class="c-veil-hint"/);
  });

  it('is not stretched from gutter to gutter by this document any more', () => {
    // The plate used to run `left: var(--gutter); right: var(--gutter)`, which
    // on a 1920 px screen is a 1888 px black band lying across the taskbar.
    // Where it stops is the system's decision now, not this document's.
    const rule = veilRule('#hint');

    expect(rule).not.toMatch(/(^|[;{\s])left\s*:/);
    expect(rule).not.toMatch(/(^|[;{\s])right\s*:/);
  });

  it('is filled at the preheat, from the catalogue', () => {
    expect(veilMain).toMatch(/hint\.textContent\s*=\s*UI_STRINGS\.veilHint/);
    expect(UI_STRINGS.veilHint.trim()).not.toBe('');
  });
});

describe("the veil's own stylesheet", () => {
  it('builds no compositor layer at parse', () => {
    // A hard constraint, not a preference: this document is PARSED during the
    // preheat, and `transform`, `backdrop-filter`, `will-change` and any
    // opacity below 1 make the compositor build a layer there and then. The
    // veil's paint would fall back into the interval lot 1d measures. It is
    // also why the plate is centred with `margin-inline: auto` and not with
    // `left: 50%; transform: translateX(-50%)`.
    const stylesheet = veilStylesheet();

    expect(stylesheet).not.toMatch(/(^|[;{\s])transform\s*:/);
    expect(stylesheet).not.toMatch(/(^|[;{\s])backdrop-filter\s*:/);
    expect(stylesheet).not.toMatch(/(^|[;{\s])will-change\s*:/);
  });
});
