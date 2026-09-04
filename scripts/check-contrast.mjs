#!/usr/bin/env node
// Measures every colour pairing src/design/tokens.css makes possible, and
// fails the build when one falls under the ratio its job requires.
//
// WHY THIS IS A SCRIPT AND NOT A REPORT
//   Lot D1 shipped ratios written in comments. A number in a comment is a
//   claim: it was true the day someone typed it, and nothing tells anyone when
//   a token moves and makes it false. The version-drift check next door exists
//   for the same reason, and for the same fact class: something copied by hand.
//   Here the numbers are RECOMPUTED from the token file on every `pnpm test`.
//
// WHAT IS MEASURED
//   WCAG 2.x relative luminance and contrast ratio, on the sRGB values the
//   tokens declare. Alpha is composited onto an explicit backdrop before
//   measuring -- a translucent colour has no ratio of its own.
//
// WHY A UNIFORM BACKDROP IS THE HONEST WORST CASE FOR GLASS
//   backdrop-filter blurs what sits behind the panel. Blurring AVERAGES the
//   backdrop, so any real screenshot composites toward its own mean, never
//   past it. A uniform pure-black and a uniform pure-white backdrop therefore
//   bracket every image this glass can ever float over. Blur cannot make the
//   composite more extreme than the extremes.
//
// WHAT IS NOT MEASURED HERE
//   Nothing is rendered. This computes what the declared values imply; it does
//   not prove the webview paints them (colour management, a display profile,
//   or Mica behind a transparent body can all shift what reaches the eye).
//   That is a lot D2 job, on a running window.
//
// No dependency, on purpose: it must run before `pnpm install` has any say.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const cssPath = join(root, 'src', 'design', 'tokens.css');

/* ------------------------------------------------------------------ *
 * 1. Read the token file. Deliberately a small hand-written walker and
 *    not a regex: a regex over CSS cannot tell a declaration inside the
 *    dark-theme block from one in the base :root, and that distinction
 *    is the whole point of measuring two themes.
 * ------------------------------------------------------------------ */

/** Returns every declaration as { path, prop, value }, in file order. */
function collect(css) {
  const src = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const out = [];
  const stack = [];
  let buf = '';
  const flush = () => {
    const text = buf.trim();
    buf = '';
    const cut = text.indexOf(':');
    if (cut < 0) return;
    out.push({
      path: stack.join(' | '),
      prop: text.slice(0, cut).trim(),
      value: text.slice(cut + 1).trim(),
    });
  };
  for (const ch of src) {
    if (ch === '{') {
      stack.push(buf.trim().replace(/\s+/g, ' '));
      buf = '';
    } else if (ch === '}') {
      flush();
      stack.pop();
    } else if (ch === ';') {
      flush();
    } else {
      buf += ch;
    }
  }
  return out;
}

const declarations = collect(readFileSync(cssPath, 'utf8'));

const DARK_PATH = '@media (prefers-color-scheme: dark) | :root';
const NO_FILTER_PATH = declarations
  .map((d) => d.path)
  .find((p) => p.startsWith('@supports not') && p.endsWith('| :root'));
const REDUCED_PATH =
  '@media (prefers-reduced-transparency: reduce) | :root';

const light = {};
const dark = {};
const noFilter = {};
const reducedTransparency = {};

for (const { path, prop, value } of declarations) {
  if (!prop.startsWith('--')) continue;
  if (path === ':root') {
    light[prop] = value;
    dark[prop] = value;
  } else if (path === DARK_PATH) {
    dark[prop] = value;
  } else if (path === NO_FILTER_PATH) {
    noFilter[prop] = value;
  } else if (path === REDUCED_PATH) {
    reducedTransparency[prop] = value;
  }
}

if (Object.keys(light).length === 0) {
  console.error(`FAIL: no :root custom properties found in ${cssPath}`);
  process.exit(1);
}

/* ------------------------------------------------------------------ *
 * 2. Colour maths.
 * ------------------------------------------------------------------ */

/** Expands var(--x) until a literal remains. */
function resolve(theme, value, depth = 0) {
  if (depth > 24) throw new Error(`var() cycle around: ${value}`);
  const match = /var\(\s*(--[\w-]+)\s*\)/.exec(value);
  if (!match) return value;
  const inner = theme[match[1]];
  if (inner === undefined) throw new Error(`unknown token ${match[1]}`);
  const next =
    value.slice(0, match.index) + inner + value.slice(match.index + match[0].length);
  return resolve(theme, next, depth + 1);
}

/** Accepts #rrggbb and the modern rgb(r g b / a) form. */
function parseColour(text) {
  const s = text.trim();
  const hex = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(s);
  if (hex) {
    return {
      r: parseInt(hex[1], 16),
      g: parseInt(hex[2], 16),
      b: parseInt(hex[3], 16),
      a: 1,
    };
  }
  const rgb =
    /^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)\s*(?:[/,]\s*([\d.]+)\s*)?\)$/i.exec(s);
  if (rgb) {
    return {
      r: Number(rgb[1]),
      g: Number(rgb[2]),
      b: Number(rgb[3]),
      a: rgb[4] === undefined ? 1 : Number(rgb[4]),
    };
  }
  throw new Error(`cannot parse colour: ${text}`);
}

const colour = (theme, token) => parseColour(resolve(theme, theme[token]));

/**
 * --glass-tint is stored as bare channels ("255 253 250") and not as a colour,
 * precisely so alpha can be swapped without touching the hue. It therefore
 * needs its own reader: it is not valid input to parseColour.
 */
function parseChannels(text) {
  const m = /^([\d.]+)\s+([\d.]+)\s+([\d.]+)$/.exec(text.trim());
  if (!m) throw new Error(`cannot parse channel triplet: ${text}`);
  return { r: Number(m[1]), g: Number(m[2]), b: Number(m[3]), a: 1 };
}

/** Source-over compositing, sRGB, as the compositor does it. */
function over(fg, bg) {
  return {
    r: fg.r * fg.a + bg.r * (1 - fg.a),
    g: fg.g * fg.a + bg.g * (1 - fg.a),
    b: fg.b * fg.a + bg.b * (1 - fg.a),
    a: 1,
  };
}

const toLinear = (channel) => {
  const c = channel / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
};

/** WCAG 2.x relative luminance. Opaque colours only -- composite first. */
function luminance({ r, g, b, a }) {
  if (a !== 1) throw new Error('luminance of a translucent colour is undefined');
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

function ratio(fg, bg) {
  const a = luminance(fg);
  const b = luminance(bg);
  const [hi, lo] = a > b ? [a, b] : [b, a];
  return (hi + 0.05) / (lo + 0.05);
}

/** Contrast against a uniform backdrop of a given relative luminance. */
const ratioAgainstLuminance = (l, other) => {
  const [hi, lo] = l > other ? [l, other] : [other, l];
  return (hi + 0.05) / (lo + 0.05);
};

const BLACK = { r: 0, g: 0, b: 0, a: 1 };
const WHITE = { r: 255, g: 255, b: 255, a: 1 };

/* ------------------------------------------------------------------ *
 * 3. The pairings, and what each one owes.
 *
 *    4.5  WCAG 1.4.3 -- text under 24px / 18.66px bold. All our body text.
 *    3.0  WCAG 1.4.11 -- boundary of a control, focus indicator.
 *    null informational: reported, never gates. Used where WCAG grants an
 *         exemption (disabled controls) or where the token is decorative.
 * ------------------------------------------------------------------ */

const SURFACES = [
  '--bg-window',
  '--surface',
  '--surface-raised',
  '--surface-inert',
  '--surface-selected',
];

const FILLS = [
  '--accent',
  '--accent-hover',
  '--accent-active',
  '--danger',
  '--warning',
  '--success',
];

const THEMES = [
  ['light', light],
  ['dark', dark],
];

const rows = [];
const record = (theme, fg, bg, value, min, note = '') =>
  rows.push({ theme, fg, bg, value, min, note });

for (const [name, theme] of THEMES) {
  for (const surface of SURFACES) {
    const bg = colour(theme, surface);
    record(name, '--text', surface, ratio(colour(theme, '--text'), bg), 4.5);
    record(name, '--text-muted', surface, ratio(colour(theme, '--text-muted'), bg), 4.5);
    // WCAG 1.4.3 exempts inactive components; PRD A4 answers with a
    // non-colour cue. Reported so the exemption stays a decision.
    record(
      name,
      '--text-inert',
      surface,
      ratio(colour(theme, '--text-inert'), bg),
      null,
      'exempt (disabled)',
    );
    record(
      name,
      '--border-strong',
      surface,
      ratio(colour(theme, '--border-strong'), bg),
      3.0,
    );
    record(
      name,
      '--focus-ring-color',
      surface,
      ratio(colour(theme, '--focus-ring-color'), bg),
      3.0,
    );
    record(name, '--border', surface, ratio(colour(theme, '--border'), bg), null, 'decorative');
  }

  for (const fill of FILLS) {
    record(
      name,
      '--text-on-solid',
      fill,
      ratio(colour(theme, '--text-on-solid'), colour(theme, fill)),
      4.5,
    );
  }

  // The semantic tokens are not only fills. src/styles.css already does
  // `color: var(--danger)` for the failure line, and PRD R2/R4 require the
  // message to be readable, so each semantic must clear 4.5:1 AS TEXT on
  // every surface it can land on. Missing this pairing is how a palette that
  // "passed" ships an unreadable error message.
  for (const semantic of ['--danger', '--warning', '--success']) {
    for (const surface of SURFACES) {
      record(
        name,
        `${semantic} (as text)`,
        surface,
        ratio(colour(theme, semantic), colour(theme, surface)),
        4.5,
      );
    }
  }
}

/*
 * The veil's resize grips, against their own two-tone ring.
 *
 * `--veil-handle` carries a promise in tokens.css: the grip wears "the same
 * two-tone stroke, so they stay findable over any pixel". That construction
 * only works if the FILL can be told apart from the RING - and that pairing was
 * never measured. The interaction spec of 4 September 2026 computed it by hand
 * and predicted a failure; a hand calculation is a direction, not a fact, so it
 * is measured here.
 *
 * 3:1 rather than 4.5:1: a grip is the boundary of a control, WCAG 1.4.11, not
 * text.
 */
for (const [name, theme] of THEMES) {
  for (const ring of ['--veil-stroke-light', '--veil-stroke-dark']) {
    record(
      name,
      '--veil-handle',
      ring,
      ratio(colour(theme, '--veil-handle'), colour(theme, ring)),
      3.0,
    );
  }
}

/* Glass: three alpha regimes, two extreme backdrops, both themes. */
const GLASS_MODES = [
  ['glass', {}],
  ['glass/no-filter', noFilter],
  ['glass/reduced-transp', reducedTransparency],
];

for (const [themeName, base] of THEMES) {
  for (const [modeName, overrides] of GLASS_MODES) {
    const theme = { ...base, ...overrides };
    const glass = parseColour(resolve(theme, theme['--glass-bg']));
    for (const [backdropName, backdrop] of [
      ['over-black', BLACK],
      ['over-white', WHITE],
    ]) {
      const composed = over(glass, backdrop);
      const label = `${modeName} ${backdropName}`;
      record(themeName, '--text', label, ratio(colour(theme, '--text'), composed), 4.5);
      record(
        themeName,
        '--text-muted',
        label,
        ratio(colour(theme, '--text-muted'), composed),
        4.5,
      );
      record(
        themeName,
        '--text-inert',
        label,
        ratio(colour(theme, '--text-inert'), composed),
        null,
        'forbidden on glass',
      );
    }
  }
}

/* ------------------------------------------------------------------ *
 * 4. The veil stroke. It is not a pairing against a token: it is drawn
 *    over the user's screen, whose content is unknown. The claim under
 *    test is that the BETTER of the two hairlines always clears 4.5:1,
 *    whatever luminance sits behind. Swept, not argued.
 * ------------------------------------------------------------------ */

const lightStroke = luminance(parseColour(light['--veil-stroke-light']));
const darkStroke = luminance(parseColour(light['--veil-stroke-dark']));

let veilWorst = Infinity;
let veilWorstAt = 0;
for (let step = 0; step <= 20000; step += 1) {
  const backdrop = step / 20000;
  const best = Math.max(
    ratioAgainstLuminance(lightStroke, backdrop),
    ratioAgainstLuminance(darkStroke, backdrop),
  );
  if (best < veilWorst) {
    veilWorst = best;
    veilWorstAt = backdrop;
  }
}
record('both', '--veil-stroke pair', 'worst backdrop luminance', veilWorst, 4.5);

/* ------------------------------------------------------------------ *
 * 5. Derived facts. These are not gates: they are the numbers the token
 *    comments cite, recomputed so a stale comment can be spotted.
 * ------------------------------------------------------------------ */

/** Lowest --glass-alpha at which --text-muted still clears 4.5:1. */
function minimumGlassAlpha(base) {
  const muted = colour(base, '--text-muted');
  const tint = parseChannels(resolve(base, base['--glass-tint']));
  const holds = (alpha) =>
    [BLACK, WHITE].every(
      (backdrop) => ratio(muted, over({ ...tint, a: alpha }, backdrop)) >= 4.5,
    );
  if (!holds(1)) return null;
  let low = 0;
  let high = 1;
  for (let i = 0; i < 40; i += 1) {
    const mid = (low + high) / 2;
    if (holds(mid)) high = mid;
    else low = mid;
  }
  return high;
}

const declaredAlpha = Number(light['--glass-alpha']);
const alphaFloors = THEMES.map(([name, theme]) => [name, minimumGlassAlpha(theme)]);

/* ------------------------------------------------------------------ *
 * 6. Report.
 * ------------------------------------------------------------------ */

const fmt = (n) => `${n.toFixed(2)}:1`;
let failures = 0;

console.log(`Contrast of ${'src/design/tokens.css'} - WCAG 2.x, recomputed\n`);
console.log(
  `  ${'THEME'.padEnd(6)}${'FOREGROUND'.padEnd(20)}${'AGAINST'.padEnd(30)}${'RATIO'.padEnd(9)}NEEDS`,
);
console.log(`  ${'-'.repeat(6 + 20 + 30 + 9 + 12)}`);

for (const { theme, fg, bg, value, min, note } of rows) {
  const ok = min === null || value >= min;
  if (!ok) failures += 1;
  const needs = min === null ? `- ${note}` : `>= ${min.toFixed(1)}  ${ok ? 'OK' : 'FAIL'}`;
  console.log(
    `  ${theme.padEnd(6)}${fg.padEnd(20)}${bg.padEnd(30)}${fmt(value).padEnd(9)}${needs}`,
  );
}

console.log('\nDERIVED FACTS (cited by the comments in tokens.css)');
console.log(
  `  veil two-tone stroke, worst case: ${fmt(veilWorst)} at backdrop luminance ` +
    `${veilWorstAt.toFixed(4)}`,
);
console.log(`  --glass-alpha declared:           ${declaredAlpha}`);
for (const [name, floor] of alphaFloors) {
  console.log(
    `  lowest alpha holding 4.5:1 (${name.padEnd(5)}): ` +
      (floor === null ? 'never holds, even opaque' : floor.toFixed(3)),
  );
}
const binding = alphaFloors
  .filter(([, floor]) => floor !== null)
  .sort((a, b) => b[1] - a[1])[0];
if (binding) {
  console.log(
    `  binding theme:                   ${binding[0]} ` +
      `(margin ${(declaredAlpha - binding[1]).toFixed(3)} of alpha)`,
  );
  if (declaredAlpha < binding[1]) {
    failures += 1;
    console.log('  ^ FAIL: the declared alpha is below that floor.');
  }
}

if (failures > 0) {
  console.error(
    `\nFAIL: ${failures} pairing(s) under the ratio their job requires.\n` +
      'Fix the token, or move the pairing to a job with a lower requirement.\n' +
      'Do not lower a threshold to make this pass.',
  );
  process.exit(1);
}

console.log(`\nOK: ${rows.length} pairings measured, none under requirement.`);
