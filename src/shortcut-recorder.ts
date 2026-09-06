/**
 * The two decisions the shortcut recorder makes, kept out of the component.
 *
 * Same rule as `window-controls.ts` and `shortcut-hint.ts`: there is no jsdom
 * and no testing-library in this project, and neither is being bought to assert
 * on markup that can be looked at directly at `#/systeme`. So whatever is worth
 * a test is a function of its arguments and lives here; `Settings.tsx` renders
 * what it is handed.
 *
 * # WHAT THIS FILE VALIDATES, AND WHAT IT DOES NOT PROTECT
 *
 * [`record`] is a convenience, not a guard. It turns a key press into an
 * accelerator and refuses the two shapes a user can produce by accident, so that
 * the screen can say why without a round trip. The REAL frontier is
 * `shortcuts::accept` in Rust: what arrives over IPC is never trusted, and a
 * devtools console, a second webview or a widened capability would all reach the
 * command without passing through this file. The two checks are deliberately
 * both there.
 *
 * # NO COMBINATION AND NO KEY CAP IS WRITTEN HERE
 *
 * The accelerator is assembled out of the press itself. What a combination is
 * DRAWN as - `Digit2` shown as « 2 », `Escape` as « Échap » - is decided in
 * `src-tauri/src/shortcuts.rs`, next to the table it has to agree with, and
 * travels back with every answer. A cap table on this side would be a second
 * statement of one rule, free to drift, with nothing able to notice.
 */

import type { ShortcutChange } from './shortcuts';
import type { StringKey } from './strings';

/**
 * The part of a `KeyboardEvent` this module reads.
 *
 * Named rather than taking `KeyboardEvent` itself, and that is what keeps the
 * test running in `environment: 'node'`: the DOM type is not available there,
 * and buying jsdom to describe four booleans and a string would be a dependency
 * for nothing.
 *
 * `code` and not `key`: a physical key code names the KEY, and the accelerator
 * Windows is asked for names it the same way. On the Belgian AZERTY this
 * application is used on, the `Digit2` key types an accented `e` unshifted, so
 * `key` would hand over a combination that changes with the active layout.
 */
export interface RecorderPress {
  readonly code: string;
  readonly ctrlKey: boolean;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
  readonly metaKey: boolean;
}

/**
 * Why a press cannot become a shortcut.
 *
 * `Extract<StringKey, …>` is not decoration: it resolves to the two literals
 * while the catalogue holds them and to a NARROWER union when it does not, so
 * the day one of those labels is renamed in `src/strings.ts` this file stops
 * compiling instead of pointing at a key that no longer exists.
 */
export type RefusalKey = Extract<
  StringKey,
  'shortcutNeedsModifier' | 'shortcutKeyUnsupported'
>;

/** What a press means while the recorder is listening. */
export type Recorded =
  /** Only modifiers are down so far. The recorder keeps waiting. */
  | { readonly kind: 'holding' }
  /** Escape: the maquette promises « Échap annule. », and this is that. */
  | { readonly kind: 'cancelled' }
  /** A press this application will not turn into a shortcut, and why. */
  | { readonly kind: 'refused'; readonly why: RefusalKey }
  /** A combination, in the plugin's syntax, ready to be offered to Rust. */
  | { readonly kind: 'combination'; readonly accelerator: string };

/**
 * The codes a bare modifier press produces.
 *
 * They are not refusals: holding Ctrl on the way to Ctrl + Maj + A is what
 * every user does, and a field that shouted at the first key would be unusable.
 *
 * `AltGraph` is absent on purpose - the AltGr key of this Belgian AZERTY reports
 * `AltRight`, which is in the list.
 */
const MODIFIER_CODES = [
  'ControlLeft',
  'ControlRight',
  'ShiftLeft',
  'ShiftRight',
  'AltLeft',
  'AltRight',
  'MetaLeft',
  'MetaRight',
] as const;

/**
 * Which flag of a press contributes which token, IN THE ORDER RUST WRITES THEM.
 *
 * The order is part of the value: `shortcuts::accept` rebuilds a canonical
 * accelerator as Ctrl, Shift, Alt, Super, and handing it the same combination
 * spelled another way would have it answer with a string this screen did not
 * send. It would still work - Rust normalises - but the two sides would be
 * saying the same thing differently for no reason.
 */
const MODIFIER_TOKENS = [
  ['ctrlKey', 'Ctrl'],
  ['shiftKey', 'Shift'],
  ['altKey', 'Alt'],
  ['metaKey', 'Super'],
] as const;

/**
 * Whether a physical key code can end a combination.
 *
 * Letters, digits and function keys, and nothing else. That is deliberately
 * NARROWER than what the plugin's parser accepts - it would take `Enter`,
 * `Space`, `ArrowUp`, a media key - and the reason is that Rust refuses every
 * key it has no cap to draw. A press this file let through and Rust then refused
 * would be a round trip spent to produce a worse message.
 */
function endsACombination(code: string): boolean {
  return (
    /^Digit[0-9]$/.test(code) || /^Key[A-Z]$/.test(code) || /^F([1-9]|1[0-9]|2[0-4])$/.test(code)
  );
}

/**
 * What one key press means to a recorder that is listening.
 *
 * The refusal that matters is `shortcutNeedsModifier`. A bare key registered
 * globally is taken from every other application on the machine - the user could
 * no longer type that character anywhere - and Windows allows it without a
 * word. Rust refuses it too; this is only what lets the screen say so at once.
 */
export function record(press: RecorderPress): Recorded {
  if (MODIFIER_CODES.some((code) => code === press.code)) {
    return { kind: 'holding' };
  }
  if (press.code === 'Escape') {
    return { kind: 'cancelled' };
  }

  const modifiers = MODIFIER_TOKENS.filter(([flag]) => press[flag]).map(([, token]) => token);

  if (modifiers.length === 0) {
    return { kind: 'refused', why: 'shortcutNeedsModifier' };
  }
  if (!endsACombination(press.code)) {
    return { kind: 'refused', why: 'shortcutKeyUnsupported' };
  }

  return { kind: 'combination', accelerator: [...modifiers, press.code].join('+') };
}

/** What the field has to say once the backend has answered. */
export type RecorderNote =
  /** The combination was taken and written down. Nothing to add. */
  | { readonly state: 'settled' }
  /**
   * It was taken and the settings file could not be written. The shortcut
   * works TODAY and will be gone after a restart, which has to be said.
   */
  | { readonly state: 'unsaved' }
  /** It was refused, and the previous combination was put back. */
  | { readonly state: 'kept'; readonly refused: readonly string[] }
  /** It was refused and nothing is registered at all. */
  | { readonly state: 'stranded'; readonly refused: readonly string[] };

/** What the recorder draws and says after one answer from the backend. */
export interface RecorderOutcome {
  /**
   * The combination the field must now show, or `undefined` when none is
   * registered.
   *
   * `undefined` is not a blank: the screen draws « Indisponible » for it, the
   * state the maquette publishes for a recorder with no combination behind it.
   */
  readonly active: readonly string[] | undefined;
  readonly note: RecorderNote;
}

/**
 * What to draw, given what the backend answered.
 *
 * # `kept` is the row this whole lot exists for
 *
 * The combination the user asked for was refused, and the one they had before is
 * REGISTERED AGAIN. Two things follow, and the second is the one an
 * implementation gets wrong: the field goes back to showing the OLD combination
 * - not the refused one, which nothing is listening for - and the message names
 * both, because « ça n'a pas marché » leaves the user unable to tell which of
 * the two they can now press.
 */
export function outcomeOf(change: ShortcutChange): RecorderOutcome {
  switch (change.outcome) {
    case 'changed':
      return {
        active: change.active.keys,
        note: { state: change.saved ? 'settled' : 'unsaved' },
      };
    case 'kept':
      return {
        active: change.active.keys,
        note: { state: 'kept', refused: change.refused.keys },
      };
    case 'stranded':
      return {
        active: undefined,
        note: { state: 'stranded', refused: change.refused.keys },
      };
  }
}

/**
 * A combination as a note writes it: one string, not chips.
 *
 * The maquette draws a combination inside a sentence as plain text rather than
 * with `Keys` (`Showcase.tsx`, the refusal specimens) - a run of keycaps inside
 * a sentence breaks the line where the sentence should not break. Same function
 * as `Launcher.tsx`'s, and it is four characters long; sharing it would cost an
 * import to save nothing.
 */
export function drawn(keys: readonly string[]): string {
  return keys.join(' + ');
}
