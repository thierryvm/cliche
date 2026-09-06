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
  /** One of them rejected: the ACL, the window guard, or a backend that failed. */
  | { readonly status: 'unreadable' };

/**
 * The states the reminder is drawn in.
 *
 * `loading`, `ready` and `refused` are the maquette's own three
 * (`src/design/Showcase.tsx`, `ShortcutState`). The other two are drawn with
 * the same `.c-note--danger` block and the system's lead-plus-clause shape; the
 * maquette has no specimen of either, because until this lot the application
 * could not tell them apart.
 */
export type ShortcutHint =
  /** The backend has not answered yet. */
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
  /** Nothing could be read, so nothing is claimed either way. */
  | { readonly state: 'unreadable' };

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

/** What to draw, given what the backend gave back. */
export function hintFor(read: RegistryRead): ShortcutHint {
  if (read.status === 'reading') {
    return { state: 'loading' };
  }
  if (read.status === 'unreadable') {
    return { state: 'unreadable' };
  }

  const { registration } = read;

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
    // name it, what is broken is the read, not the shortcut.
    return keys === undefined ? { state: 'unreadable' } : { state: 'ready', keys };
  }

  return keys === undefined ? { state: 'unavailable' } : { state: 'refused', keys };
}
