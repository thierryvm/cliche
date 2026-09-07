/**
 * What the launcher says about the capture shortcut, decided outside React.
 *
 * Same reason as `window-controls.ts`: the round trip to the backend cannot be
 * exercised here, so the part that can be - the mapping from what came back to
 * what is drawn - is a function of its argument.
 *
 * # NO COMBINATION IS WRITTEN IN THIS FILE
 *
 * Not even as a fallback. Rust is what registers a combination with Windows
 * (`src-tauri/src/shortcuts.rs` has the long version), and a default typed here
 * would be a second table free to drift from what the operating system was
 * actually asked for. When the registry cannot be read, the launcher says so;
 * it does not guess.
 *
 * # WHAT CHANGED ON 6 SEPTEMBER 2026
 *
 * This module used to have one input - the registry table - and it drew
 * `refused` whenever it could not find a capture entry in it. The comment where
 * that is now explained said the state was "waiting" for Rust to carry the
 * registration outcome across; `describe_shortcut_status` is that, so the
 * decision is now made on WHAT THE SYSTEM ANSWERED and no longer on the shape
 * of a table.
 *
 * The consequence worth naming: `refused` now means refused, by Windows, for a
 * combination this file can name. Everything that merely leaves the user
 * without a shortcut is `unavailable`, and everything that leaves this file
 * unable to say anything at all is `unreadable`. Three sentences that used to
 * be one, because they are not the same fact and the middle one used to be
 * told as the third.
 *
 * # WHAT CHANGED ON 7 SEPTEMBER 2026, and it is a fourth sentence
 *
 * `starting` - the answer Rust now gives while `setup` is still running - is
 * drawn as `loading`, NOT as a failure. Until that day the same machine state
 * arrived here as an `invoke` that had rejected, and this module drew the red
 * « le registre des raccourcis n'a pas pu être lu » over a shortcut that then
 * worked: the launcher boots and asks while `setup` is still building the
 * veil's WebView2 (`src-tauri/src/shortcut.rs` has the long version). « Pas
 * encore » is not « échec ». What makes `loading` end rather than last forever
 * is `src/shortcut-probe.ts`, which decides when to ask again.
 *
 * And when the read really does reject, `unreadable` now CARRIES what rejected.
 * Thierry decided the shape that day: a French sentence from the catalogue,
 * then the technical reason as it arrived, in English. It is the shape the
 * veil's failure toast already had (`src/veil/confirmation.ts`) and the one the
 * monitor read-out already had (`src/DisplaysProbe.tsx`), so it is a precedent
 * followed rather than a second way of saying that something failed.
 */

import type { ShortcutEntry, ShortcutStatus } from './shortcuts';
import type { StringKey } from './strings';

/** Where the launcher's read of the backend stands. */
export type RegistryRead =
  /** The calls are still in flight. */
  | { readonly status: 'reading' }
  /**
   * Both came back: the table this application intends to hold, and what the
   * operating system answered about the capture combination in it.
   */
  | {
      readonly status: 'read';
      readonly entries: readonly ShortcutEntry[];
      readonly registration: ShortcutStatus;
    }
  /**
   * One of them rejected: the ACL, the window guard, or a backend that failed.
   *
   * `reason` is what rejected, in its own words - English, technical, and
   * normalised by `reasonText` in `src/shortcut-probe.ts`. It is `''` when the
   * rejection carried nothing readable, which is a case with a drawing of its
   * own rather than a separator left hanging.
   */
  | { readonly status: 'unreadable'; readonly reason: string };

/**
 * The states the reminder is drawn in.
 *
 * `loading`, `ready` and `refused` are the maquette's own three
 * (`src/design/Showcase.tsx`, `ShortcutState`), and `unreadable` is the fourth
 * specimen of section `s-hint`, published on 7 September 2026 with the
 * quotation it now carries. `unavailable` is the one with no drawing of its
 * own: it is the same `.c-note--danger` block and the same lead-plus-clause
 * shape, and it has none because until the lot of 6 September the application
 * could not tell it apart from its neighbours.
 */
export type ShortcutHint =
  /**
   * The backend has not answered yet, or has answered that it has not decided.
   * Both are « pas encore » and neither is a failure.
   */
  | { readonly state: 'loading' }
  /** The shortcut works, and these are the keys to press. */
  | { readonly state: 'ready'; readonly keys: readonly string[] }
  /**
   * Windows refused this combination. The keys are carried because the note
   * NAMES the combination - a refusal the user cannot see is a refusal the user
   * cannot act on (PRD R4).
   */
  | { readonly state: 'refused'; readonly keys: readonly string[] }
  /**
   * There is no shortcut, and no combination can honestly be named as the one
   * that was refused: it was never offered, or what came back does not describe
   * the combination the system answered about.
   */
  | { readonly state: 'unavailable' }
  /**
   * Nothing could be read, so nothing is claimed either way - and what stopped
   * the read is quoted when there is something to quote.
   *
   * Two fields rather than one sentence, for the reason `window-controls.ts`
   * gives: the French belongs to `src/strings.ts`, which the showcase decides,
   * and a module that assembled it here would be a second place a label could be
   * written. `Extract<StringKey, …>` narrows to nothing the day either form
   * leaves the catalogue, so this file stops compiling instead of pointing at a
   * key that is gone.
   *
   * `quotation` carries the space that separates it from the sentence, the way
   * `planFor` carries the one before « copié » in `src/veil/confirmation.ts`:
   * the punctuation belongs to the assembled sentence, and an empty quotation
   * has to leave NOTHING behind, not a trailing space.
   */
  | {
      readonly state: 'unreadable';
      readonly sentenceKey: Extract<
        StringKey,
        'shortcutRegistryUnreadable' | 'shortcutRegistryUnreadableWithReason'
      >;
      readonly quotation: string;
    };

/**
 * Which registry entry the launcher's headline action shares its shortcut with.
 *
 * Matched on `descriptionKey` and not on `id`, and the reason is that this
 * literal is CHECKED: `Extract<StringKey, …>` narrows to nothing the day
 * `captureRegion` leaves `src/strings.ts`, and `captureRegion` is also the
 * label of the primary tile, so the tile and the reminder under it cannot come
 * to mean two different actions. An `id` would be a bare string agreeing with
 * `shortcuts.rs` by hand, with nothing in either language to hold it.
 */
const CAPTURE_REGION: Extract<StringKey, 'captureRegion'> = 'captureRegion';

/**
 * The keys to draw for the combination the system answered about, if the two
 * can be shown to be the same combination.
 *
 * The accelerator comparison is the whole point. Both values come from the one
 * static table in `shortcuts.rs` today, so they cannot disagree - but the day
 * they can, drawing the entry's keys under a sentence about the status would be
 * naming the wrong combination to a user trying to free it. `undefined` says
 * "nothing here may be drawn as that combination", which the callers turn into
 * a state that names none.
 */
function keysFor(
  entries: readonly ShortcutEntry[],
  accelerator: string,
): readonly string[] | undefined {
  const capture = entries.find((entry) => entry.descriptionKey === CAPTURE_REGION);

  if (capture === undefined || capture.accelerator !== accelerator) {
    return undefined;
  }
  // An entry with no key would draw a sentence promising a shortcut and no cap
  // to press, which reads as an interface bug rather than as a missing feature.
  return capture.keys.length === 0 ? undefined : capture.keys;
}

/**
 * The note that claims nothing, drawn with or without a quotation.
 *
 * The trim, and the branch it feeds, are `planFor`'s in
 * `src/veil/confirmation.ts`: a reason made of nothing but spaces would leave
 * « Le registre des raccourcis n'a pas pu être lu : » with an empty tail, which
 * reads as a sentence the interface failed to finish.
 */
function unreadable(reason: string): ShortcutHint {
  const quoted = reason.trim();

  if (quoted === '') {
    return { state: 'unreadable', sentenceKey: 'shortcutRegistryUnreadable', quotation: '' };
  }
  return {
    state: 'unreadable',
    sentenceKey: 'shortcutRegistryUnreadableWithReason',
    quotation: ` ${quoted}`,
  };
}

/** What to draw, given what the backend gave back. */
export function hintFor(read: RegistryRead): ShortcutHint {
  if (read.status === 'reading') {
    return { state: 'loading' };
  }
  if (read.status === 'unreadable') {
    return unreadable(read.reason);
  }

  const { registration } = read;

  if (registration.status === 'starting') {
    // NOT a failure: `setup` has not decided yet, and neither has this screen.
    // The table that came back with it is not read either - until `install` has
    // run, `describe_shortcuts` hands back the combination the SOURCE ships
    // with, which is not necessarily the one this launch will offer.
    return { state: 'loading' };
  }

  if (registration.status === 'not-attempted') {
    // Nothing external refused anything: there is simply no shortcut. The
    // reason is for the console - it names a fault of this application, in
    // English, and the screen is neither the place nor the language for it.
    return { state: 'unavailable' };
  }

  const keys = keysFor(read.entries, registration.accelerator);

  if (registration.status === 'accepted') {
    // The system took the combination, so the shortcut WORKS - and saying it
    // does not would be the worst of the five answers. When the table cannot
    // name it, what is broken is the read, not the shortcut - and NOTHING
    // rejected, so there is no backend sentence to quote.
    return keys === undefined ? unreadable('') : { state: 'ready', keys };
  }

  return keys === undefined ? { state: 'unavailable' } : { state: 'refused', keys };
}
