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
 * Asks the backend for the shortcut registry.
 *
 * Rejects with the backend's error string; the ACL and `ipc::ensure_from` both
 * refuse this command to any window but `main`.
 */
export function describeShortcuts(): Promise<ShortcutEntry[]> {
  return invoke<ShortcutEntry[]>('describe_shortcuts');
}
