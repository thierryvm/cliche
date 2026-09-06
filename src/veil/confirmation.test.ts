/**
 * What the veil does once the clipboard has answered, under test.
 *
 * Same reasoning as `zones.test.ts`, and the same constraint: this repository
 * has no jsdom and buys none, so the part that deserves a test is the part that
 * touches nothing but its arguments. `confirmation.ts` is that part - the
 * decision - and `main.ts` keeps the two lines that apply it to a document and
 * to a window.
 *
 * # THE ONE PROPERTY THIS FILE EXISTS FOR
 *
 * A refusal must NEVER make the veil stop answering the pointer. The failure
 * toast carries its own way out - a 44 px button - and a click-through window
 * would put that button behind glass: the message would name what went wrong
 * and offer a control nobody can reach. Success is the opposite case and the
 * only one allowed to let the cursor through.
 */

import { describe, expect, it } from 'vitest';

import { millisecondsIn, planFor, sizeLabel } from './confirmation';
import { UI_STRINGS } from '../strings';

/** The two token values as `src/design/tokens.css` declares them today. */
const TIMING = { dwellMs: 2400, fadeMs: 160 } as const;

/** The same two under `prefers-reduced-motion`, where only the fade collapses. */
const REDUCED = { dwellMs: 2400, fadeMs: 0 } as const;

describe('millisecondsIn', () => {
  it('reads both units CSS is allowed to answer with', () => {
    // `getPropertyValue` hands back a custom property as it was WRITTEN, so a
    // token declared in seconds arrives in seconds. Both spellings are the
    // token file's own: --dur-toast-dwell is `2400ms`, --dur-short falls to
    // `0s` under prefers-reduced-motion.
    expect(millisecondsIn('2400ms')).toBe(2400);
    expect(millisecondsIn('2.4s')).toBe(2400);
    expect(millisecondsIn('160ms')).toBe(160);
  });

  it('accepts a zero duration, which is what reduced motion asks for', () => {
    // `lengthInPixels` refuses zero because a zero-width grip ring is a bug.
    // A zero-length ANIMATION is a deliberate token value, and refusing it here
    // would throw at load for exactly the users who asked for less movement.
    expect(millisecondsIn('0s')).toBe(0);
    expect(millisecondsIn('0ms')).toBe(0);
  });

  it('is not fooled by the s at the end of ms', () => {
    // The bug this test exists for: testing for `s` before `ms` reads `160ms`
    // as 160 SECONDS, and the confirmation would sit on the user's screen for
    // two and a half minutes.
    expect(millisecondsIn('160ms')).toBeLessThan(1000);
  });

  it('ignores the whitespace a computed value can carry', () => {
    expect(millisecondsIn(' 2400ms ')).toBe(2400);
  });

  it('throws rather than guess when the unit is missing or unknown', () => {
    // A fallback number would be the second copy of a token that tokens.css
    // opens by forbidding - and a plausible one nobody would ever look at.
    expect(() => millisecondsIn('2400')).toThrow();
    expect(() => millisecondsIn('fast')).toThrow();
    expect(() => millisecondsIn('')).toThrow();
    expect(() => millisecondsIn('-1s')).toThrow();
  });
});

describe('sizeLabel', () => {
  it('writes the two dimensions with the multiplication sign the maquette uses', () => {
    // U+00D7, not the letter x. `src/design/Showcase.tsx` publishes
    // `933×577`, and `.c-num` sets it in the numeric face.
    expect(sizeLabel({ width: 933, height: 577 })).toBe('933×577');
  });

  it('keeps width first, so a portrait cut is not reported as a landscape one', () => {
    expect(sizeLabel({ width: 8, height: 64 })).toBe('8×64');
  });

  it('answers null for anything that does not describe a size', () => {
    // This value crosses the IPC frontier, so `invoke<T>` is a claim
    // TypeScript cannot check. Null means "say `copié` and name no figure",
    // which is a smaller lie than a fabricated one.
    expect(sizeLabel(undefined)).toBeNull();
    expect(sizeLabel(null)).toBeNull();
    expect(sizeLabel('933x577')).toBeNull();
    expect(sizeLabel({ width: 933 })).toBeNull();
    expect(sizeLabel({ width: 0, height: 577 })).toBeNull();
    expect(sizeLabel({ width: -1, height: 577 })).toBeNull();
    expect(sizeLabel({ width: 933.5, height: 577 })).toBeNull();
    expect(sizeLabel({ width: '933', height: '577' })).toBeNull();
  });
});

describe('planFor - a copy that worked', () => {
  const plan = planFor({ copied: true, size: '933×577' }, TIMING);

  it('is the maquette markup, class for class', () => {
    // `src/design/Showcase.tsx`, section s-toast. The showcase draws the
    // specimen WITHOUT --transient so it can be looked at; the real one has it,
    // which is what makes the message leave on its own.
    expect(plan.classes).toEqual([
      'c-note',
      'c-note--success',
      'c-toast',
      'c-toast--transient',
    ]);
  });

  it('is announced as a status and not as an alert', () => {
    // role=status is polite: it does not interrupt, because nothing went
    // wrong. The failure below is the one entitled to interrupt.
    expect(plan.role).toBe('status');
  });

  it('owns the screen for the dwell AND its way out, never less', () => {
    // The window is taken down when this elapses, so a lifetime of the dwell
    // ALONE would cut the exit animation off at its first frame.
    expect(plan.dwellMs).toBe(2400 + 160);
    expect(planFor({ copied: true, size: null }, REDUCED).dwellMs).toBe(2400);
  });

  it('lets the cursor through, because the veil has nothing left to do', () => {
    expect(plan.clickThrough).toBe(true);
  });

  it('carries no way out of its own: it leaves on its own', () => {
    expect(plan.dismissable).toBe(false);
  });

  it('names the size, then the word the showcase publishes', () => {
    expect(plan.measurement).toBe('933×577');
    expect(plan.lead).toBeNull();
    expect(plan.text).toBe(` ${UI_STRINGS.copied}`);
  });

  it('drops the leading space when there is no figure to precede it', () => {
    const unnamed = planFor({ copied: true, size: null }, TIMING);

    expect(unnamed.measurement).toBeNull();
    expect(unnamed.text).toBe(UI_STRINGS.copied);
  });
});

describe('planFor - a copy that did not happen', () => {
  const refusal = 'the clipboard refused the image: it is held by another application';
  const plan = planFor({ copied: false, reason: refusal }, TIMING);

  it('is the maquette markup, class for class, and never transient', () => {
    // components.css: "A failure has NO transient variant... a message nobody
    // managed to read names nothing".
    expect(plan.classes).toEqual(['c-note', 'c-note--danger', 'c-toast']);
    expect(plan.classes).not.toContain('c-toast--transient');
  });

  it('is announced as an alert, because it interrupts on purpose', () => {
    expect(plan.role).toBe('alert');
  });

  it('never leaves on its own', () => {
    expect(plan.dwellMs).toBeNull();
  });

  it('KEEPS the pointer, or its own dismiss button is unreachable', () => {
    // The property this file exists for. A click-through veil showing a
    // failure is a message with a button behind glass.
    expect(plan.clickThrough).toBe(false);
    expect(plan.dismissable).toBe(true);
  });

  it('leads with the word, then carries the reason unaltered', () => {
    // The lead is catalogued; the reason is whatever Rust said, and it is
    // shown rather than reworded. Naming a cause we have not established -
    // "another application holds the clipboard" - for a refusal that was
    // actually "this selection is too small" would be a sentence that lies.
    expect(plan.measurement).toBeNull();
    expect(plan.lead).toBe(UI_STRINGS.failure);
    expect(plan.text).toBe(` — ${refusal}`);
  });

  it('drops the separator rather than dangle it when no reason arrives', () => {
    // A refusal with nothing to say is still a refusal, and the lead alone
    // says it. What must NOT appear is `Échec — ` with nothing after the dash,
    // which reads as a sentence the interface failed to finish. No invented
    // cause either: any wording put here would be a French label that
    // `src/strings.ts` does not hold and the showcase never published.
    const empty = planFor({ copied: false, reason: '   ' }, TIMING);

    expect(empty.lead).toBe(UI_STRINGS.failure);
    expect(empty.text).toBe('');
  });
});
