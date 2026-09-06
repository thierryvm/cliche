import { invoke } from '@tauri-apps/api/core';

/**
 * One shortcut, as the Rust registry states it.
 *
 * Field names mirror `ShortcutEntry` in `src-tauri/src/shortcuts.rs`, which
 * serialises with `rename_all = "camelCase"`.
 *
 * THIS FILE HOLDS NO SHORTCUT DATA, and that is its whole design. Rust is what
 * registers a combination with the operating system, in `setup`, before this
 * webview exists; a table repeated here could not be checked against what
 * Windows was actually asked for, and would be free to drift into announcing a
 * combination nothing listens for. The header of `shortcuts.rs` has the long
 * version.
 */
export interface ShortcutEntry {
  /** Stable identifier, e.g. `capture-region`. Never shown to the user. */
  readonly id: string;
  /** The combination in the plugin's syntax, e.g. `Ctrl+Shift+Digit2`. */
  readonly accelerator: string;
  /** The same combination as it is DRAWN, one chip per key: `["Ctrl","Maj","2"]`. */
  readonly keys: readonly string[];
  /**
   * Key into `UI_STRINGS` in `src/strings.ts`: what the shortcut does.
   *
   * Typed `string` rather than `StringKey`, deliberately: this value crosses
   * the IPC boundary at run time, and narrowing it to a union would be a claim
   * TypeScript never gets to check. What keeps it inside the catalogue is a
   * test that reads both files - `every_description_key_exists_in_the_string_catalogue`,
   * in `src-tauri/src/shortcuts.rs`.
   */
  readonly descriptionKey: string;
  /** The help-page heading this entry files itself under. */
  readonly category: ShortcutCategory;
}

/** The closed list of help-page headings, mirroring `ShortcutCategory` in Rust. */
export type ShortcutCategory = 'capture';

/**
 * What the operating system ANSWERED when this application offered it the
 * capture combination, at startup, on this machine.
 *
 * Mirrors `ShortcutStatus` in `src-tauri/src/shortcut.rs`, which serialises
 * internally tagged on `status`. The three literals below are the ones Rust is
 * told to emit, and a Rust test - `the_three_wire_tags_are_the_ones_serde_is_
 * told_to_emit_and_the_frontend_reads` - reads THIS file to hold the two sides
 * together. TypeScript cannot: the value arrives at run time.
 *
 * # The registry is a promise, this is what became of it
 *
 * `describeShortcuts` above says which combinations this application INTENDS to
 * hold. That table is the same on every machine. This says whether Windows let
 * it, which is not - and drawing a reminder from the first alone is how a
 * launcher comes to announce a combination nothing is listening for.
 *
 * `reason` is the operating system's own words, or this application's, in
 * English. It belongs in a console, never on the screen: what the screen says
 * is decided in `src/shortcut-hint.ts`, from the catalogue.
 */
export type ShortcutStatus =
  /** The system took the combination. The shortcut works. */
  | { readonly status: 'accepted'; readonly accelerator: string }
  /** The system refused it - another program is holding it. */
  | { readonly status: 'refused-by-system'; readonly accelerator: string; readonly reason: string }
  /**
   * It was never offered: an unreadable combination, an entry missing from the
   * registry, a plugin that failed to load. No `accelerator`, because in the
   * worst of those cases there is no entry to read one from.
   */
  | { readonly status: 'not-attempted'; readonly reason: string };

/**
 * Asks the backend for the shortcut registry.
 *
 * Rejects with the backend's error string; the ACL and `ipc::ensure_from` both
 * refuse this command to any window but `main`.
 */
export function describeShortcuts(): Promise<ShortcutEntry[]> {
  return invoke<ShortcutEntry[]>('describe_shortcuts');
}

/**
 * Asks the backend what the system answered about the capture shortcut.
 *
 * Rejects the same way as its neighbour, and one case is worth naming: it also
 * rejects when no status is managed at all, which means `setup` never got as
 * far as installing the shortcut.
 */
export function describeShortcutStatus(): Promise<ShortcutStatus> {
  return invoke<ShortcutStatus>('describe_shortcut_status');
}
