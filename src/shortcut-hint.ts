/**
 * What the launcher says about the capture shortcut, decided outside React.
 *
 * Same reason as `window-controls.ts`: the round trip to `describe_shortcuts`
 * cannot be exercised here, so the part that can be - the mapping from what
 * came back to what is drawn - is a function of its argument.
 *
 * # NO COMBINATION IS WRITTEN IN THIS FILE
 *
 * Not even as a fallback. Rust is what registers a combination with Windows
 * (`src-tauri/src/shortcuts.rs` has the long version), and a default typed here
 * would be a second table free to drift from what the operating system was
 * actually asked for. When the registry cannot be read, the launcher says so;
 * it does not guess.
 */

import type { ShortcutEntry } from './shortcuts';
import type { StringKey } from './strings';

/** Where the read of the shortcut registry stands. */
export type RegistryRead =
  /** The `describe_shortcuts` call is still in flight. */
  | { readonly status: 'reading' }
  /** It came back. The list may still not describe a region capture. */
  | { readonly status: 'read'; readonly entries: readonly ShortcutEntry[] }
  /** It rejected: the ACL, the window guard, or a backend that failed. */
  | { readonly status: 'unreadable' };

/**
 * The three states the reminder is drawn in, named as the design system names
 * them (`src/design/Showcase.tsx`, `ShortcutState`).
 *
 * `refused` IS THE MAQUETTE'S NAME AND NOT THE WHOLE TRUTH, and that is worth
 * one paragraph rather than a surprise later. The showcase draws `refused` as
 * "this combination is held by another application" - a fact that lives in
 * `shortcut::install`'s return value and reaches no webview: `describe_shortcuts`
 * hands back the static table whether Windows accepted the combination or not.
 * So what this module can actually distinguish is a registry it could not read
 * or that describes no region capture, and that - not a refusal by Windows - is
 * what `Launcher.tsx` says out loud. Carrying the registration outcome into the
 * registry is a change to Rust, and it is what this state is waiting for.
 */
export type ShortcutHint =
  | { readonly state: 'loading' }
  | { readonly state: 'ready'; readonly keys: readonly string[] }
  | { readonly state: 'refused' };

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

/** What to draw, given what the backend gave back. */
export function hintFor(read: RegistryRead): ShortcutHint {
  if (read.status === 'reading') {
    return { state: 'loading' };
  }
  if (read.status === 'unreadable') {
    return { state: 'refused' };
  }

  const capture = read.entries.find((entry) => entry.descriptionKey === CAPTURE_REGION);

  // An entry with no key would draw a sentence promising a shortcut and no cap
  // to press, which reads as an interface bug rather than as a missing feature.
  if (capture === undefined || capture.keys.length === 0) {
    return { state: 'refused' };
  }

  return { state: 'ready', keys: capture.keys };
}
