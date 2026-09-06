/**
 * WHAT THE VEIL DOES ONCE THE CLIPBOARD HAS ANSWERED: pure, DOM-free, tested.
 *
 * Split out of `main.ts` for the reason `zones.ts` was, and it is structural
 * rather than tidiness: `main.ts` queries the DOM at module load, so a Node test
 * process cannot import it. Everything below touches nothing but its arguments,
 * which is what puts the decision under test with no simulated DOM and no
 * dependency bought to provide one.
 *
 * # THE DECISION, IN ONE SENTENCE
 *
 * A copy that worked steps the veil OUT OF THE WAY - it goes transparent, stops
 * answering the pointer, shows a confirmation for the length of
 * `--dur-toast-dwell` and then closes. A copy that did not happen leaves the
 * veil exactly where it is, ANSWERING THE POINTER, until the user dismisses the
 * message or presses Escape.
 *
 * The asymmetry is not a preference. The failure toast carries its own way out -
 * a 44 px button, `.c-toast__dismiss` - and a click-through window would put
 * that button behind glass: a message that names what went wrong and offers a
 * control nobody can reach. That is the property `confirmation.test.ts` exists
 * for.
 *
 * # NO WORDING IS DECIDED HERE EITHER
 *
 * Both labels come from `src/strings.ts`, which copies them from
 * `src/design/Showcase.tsx`, which is where they were looked at and signed off.
 * `scripts/check-strings.mjs` fails the suite the day the two disagree, and the
 * day somebody re-types one of them under `src/`.
 *
 * What is NOT catalogued is the REASON a copy failed: it arrives from Rust, in
 * English, as whatever `clipboard::copy_selection` or the area rule said. It is
 * shown as it arrived. The alternative - the maquette's own specimen sentence,
 * "une autre application le tient ouvert" - would name a cause nothing has
 * established, and would be flatly false for the refusal the user meets most
 * often, which is a selection too small to be a deliberate drag.
 *
 * # NOTHING HERE IS INSIDE THE 150 ms BUDGET
 *
 * That budget ends at `painted`, before the user has started dragging. Every
 * function below runs after they have finished, at human speed.
 */

import { UI_STRINGS } from '../strings';

/**
 * U+00D7 MULTIPLICATION SIGN, and not the letter x.
 *
 * The code point is named here because the two are indistinguishable in a
 * source file, and `.c-num` sets this string in the numeric face, where the
 * difference would be blamed on the font. `src/design/Showcase.tsx` publishes
 * `933×577`, and `confirmation.test.ts` holds this constant against that
 * spelling.
 */
const TIMES = '×';

/**
 * U+2014 EM DASH, the separator the system's lead-plus-clause shape uses - the
 * same one `.c-note--danger` wears in the showcase. Same reasoning as above: a
 * hyphen-minus typed in its place would be a silent regression, so the code
 * point is named and the test asserts the character.
 */
const EM_DASH = '—';

/** What came back from `veil_selected`, once it has been read. */
export type CopyOutcome =
  /**
   * The bytes are on the clipboard. `size` is the cut's dimensions in physical
   * pixels, or `null` when the reply did not describe them - see [`sizeLabel`].
   */
  | { readonly copied: true; readonly size: string | null }
  /** Nothing was copied. `reason` is what Rust said, in Rust's own words. */
  | { readonly copied: false; readonly reason: string };

/**
 * The two duration tokens, resolved from CSS by the caller.
 *
 * Passed in rather than read here, because reading them needs
 * `getComputedStyle` and this file is the half that has no window. They are
 * `--dur-toast-dwell` and `--dur-short`; neither number is written in
 * TypeScript, for the reason `tokens.css` opens by giving.
 */
export interface ToastTiming {
  /** How long the confirmation stays readable: `--dur-toast-dwell`. */
  readonly dwellMs: number;
  /** How long it then takes to leave: `--dur-short`, zero under reduced motion. */
  readonly fadeMs: number;
}

/** What to draw, for how long, and what the window does meanwhile. */
export interface ToastPlan {
  /** The class list, exactly as `src/design/Showcase.tsx` writes it. */
  readonly classes: readonly string[];
  /** The live region's manners: `status` waits its turn, `alert` interrupts. */
  readonly role: 'status' | 'alert';
  /**
   * How long the toast owns the screen before the veil is taken down, or `null`
   * when it never leaves on its own.
   *
   * The dwell AND the exit: the window disappears when this elapses, so a
   * lifetime of the dwell alone would cut `c-toast-out` off at its first frame.
   */
  readonly dwellMs: number | null;
  /** Whether the window stops answering the pointer. */
  readonly clickThrough: boolean;
  /** Whether the toast carries a control that dismisses it. */
  readonly dismissable: boolean;
  /** The tabular figure that opens the message, or `null` when there is none. */
  readonly measurement: string | null;
  /** The bold lead of the message, or `null` when it has none. */
  readonly lead: string | null;
  /** The rest of the message, separator included. */
  readonly text: string;
}

/**
 * Reads a duration token as a number of milliseconds.
 *
 * The sibling of `lengthInPixels` in `./zones`, and it exists for the same
 * reason: a custom property is substituted AS WRITTEN, so `--dur-toast-dwell`
 * arrives here as `2400ms` and `--dur-short` as `160ms` - or as `0s` under
 * `prefers-reduced-motion`.
 *
 * `ms` is tested BEFORE `s`, and that order is the whole of the parsing: `160ms`
 * ends with `s` as well, so the obvious order reads it as 160 SECONDS and the
 * confirmation sits on the user's screen for two and a half minutes.
 *
 * Zero is accepted, unlike a length: a zero-duration animation is a deliberate
 * token value under reduced motion, and throwing on it would break the page at
 * load for exactly the users who asked for less movement.
 *
 * Anything else THROWS, at load, during the preheat. A fallback number would be
 * a second copy of a token - what `tokens.css` opens by forbidding - and a
 * plausible one nobody would ever look at again.
 */
export const millisecondsIn = (raw: string): number => {
  const text = raw.trim();
  const value = Number.parseFloat(text);

  if (!Number.isFinite(value) || value < 0) {
    throw new Error(`"${raw}" is not a duration`);
  }
  if (text.endsWith('ms')) {
    return value;
  }
  if (text.endsWith('s')) {
    return value * 1000;
  }

  throw new Error(`"${raw}" is a duration in a unit this page cannot resolve`);
};

/** A dimension in physical pixels: whole, and at least one. */
const isPixelCount = (value: unknown): value is number =>
  typeof value === 'number' && Number.isInteger(value) && value > 0;

/**
 * Turns `veil_selected`'s reply into the figure the confirmation shows.
 *
 * IT IS VALIDATED RATHER THAN CAST, and that is the point of the function.
 * `invoke<T>()` is an assertion TypeScript cannot check: the value crossed an
 * IPC frontier, was serialised by serde and parsed by `JSON.parse`. A cast would
 * let `undefined×undefined` reach the screen as a measurement.
 *
 * `null` means "say the word and name no figure". That is a smaller lie than a
 * fabricated size, and the copy really did succeed - the reply is the only part
 * that is in doubt.
 */
export const sizeLabel = (reply: unknown): string | null => {
  if (typeof reply !== 'object' || reply === null) {
    return null;
  }
  if (!('width' in reply) || !('height' in reply)) {
    return null;
  }

  const { width, height } = reply;
  if (!isPixelCount(width) || !isPixelCount(height)) {
    return null;
  }

  return `${width}${TIMES}${height}`;
};

/** What the veil shows, and becomes, for a given outcome. */
export const planFor = (outcome: CopyOutcome, timing: ToastTiming): ToastPlan => {
  if (outcome.copied) {
    return {
      // `--transient` is the class the showcase deliberately LEAVES OFF its
      // specimen, so the confirmation can be looked at; the real one wears it,
      // and it is what makes the message leave without being asked.
      classes: ['c-note', 'c-note--success', 'c-toast', 'c-toast--transient'],
      role: 'status',
      dwellMs: timing.dwellMs + timing.fadeMs,
      // The capture is over. A veil that still answered the pointer would be a
      // full-screen sheet of glass between the user and the desktop they can
      // once again see through it.
      clickThrough: true,
      dismissable: false,
      measurement: outcome.size,
      lead: null,
      // The space belongs to the sentence, not to the label: the showcase
      // writes `<span class="c-num">933×577</span> copié`.
      text: outcome.size === null ? UI_STRINGS.copied : ` ${UI_STRINGS.copied}`,
    };
  }

  const reason = outcome.reason.trim();

  return {
    classes: ['c-note', 'c-note--danger', 'c-toast'],
    role: 'alert',
    // components.css: "A failure has NO transient variant... a message nobody
    // managed to read names nothing, which is exactly what PRD R2 forbids."
    dwellMs: null,
    clickThrough: false,
    dismissable: true,
    measurement: null,
    lead: UI_STRINGS.failure,
    // No dangling separator when nothing came back: `Échec — ` with an empty
    // tail reads as a sentence the interface failed to finish. And no invented
    // cause in its place - any wording put here would be a French label
    // `src/strings.ts` does not hold and the showcase never published.
    text: reason === '' ? '' : ` ${EM_DASH} ${reason}`,
  };
};
