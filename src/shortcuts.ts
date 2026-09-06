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
 * A combination, and the caps that draw it.
 *
 * Mirrors `Combination` in `src-tauri/src/shortcuts.rs`. The two travel
 * together because for a combination the USER chose there is no hand-written
 * registry row to read the caps from: Rust derives them once, when it accepts
 * the combination, and everything downstream carries what it derived. Nothing
 * on this side turns an accelerator into key caps — that rule lives next to the
 * table it has to agree with.
 */
export interface Combination {
  /** The combination in the plugin's syntax, e.g. `Ctrl+Shift+Digit2`. */
  readonly accelerator: string;
  /** The same combination as it is DRAWN, one chip per key. */
  readonly keys: readonly string[];
}

/**
 * What became of a request to change the capture combination.
 *
 * Mirrors `ShortcutChange` in `src-tauri/src/shortcut.rs`, tagged on `outcome`.
 * The three literals are the ones Rust is told to emit, and a Rust test —
 * `the_three_change_tags_are_the_ones_serde_emits_and_the_frontend_reads` —
 * reads THIS file to hold the two sides together. TypeScript cannot: the value
 * arrives at run time.
 *
 * # `kept` is the one that matters
 *
 * A combination the operating system refuses does NOT leave this application
 * without a shortcut: the previous one is registered again, at once, and this
 * answer names both. `reason` is the system's own words, in English, for a
 * console — what the screen says is decided from the catalogue, in
 * `src/Settings.tsx`.
 */
export type ShortcutChange =
  /** The combination asked for is the one that is live. */
  | {
      readonly outcome: 'changed';
      readonly active: Combination;
      /**
       * Whether the choice will survive a restart. `false` means the shortcut
       * WORKS and the settings file could not be written — which has to be said,
       * or the user finds the old combination back tomorrow with nothing having
       * warned them.
       */
      readonly saved: boolean;
    }
  /** It was refused, and the PREVIOUS combination is still live. */
  | {
      readonly outcome: 'kept';
      readonly refused: Combination;
      readonly active: Combination;
      readonly reason: string;
    }
  /** It was refused and no combination is live at all. */
  | { readonly outcome: 'stranded'; readonly refused: Combination; readonly reason: string };

/**
 * Asks the backend for the shortcut registry.
 *
 * What comes back is the table with the CAPTURE row set to the combination this
 * launch really offered the operating system — see `rows` in
 * `src-tauri/src/shortcuts.rs`. That substitution is why nothing on this side
 * knows a shortcut is settable at all: the help page and the launcher's reminder
 * go on reading one table.
 *
 * Rejects with the backend's error string; the ACL and `ipc::ensure_from` both
 * refuse this command to any window but `main`.
 */
export function describeShortcuts(): Promise<ShortcutEntry[]> {
  return invoke<ShortcutEntry[]>('describe_shortcuts');
}

/**
 * Asks the backend to take a new capture combination, NOW.
 *
 * The old one is released and the new one taken before this resolves. A
 * combination the system refuses comes back as `kept`, with the previous one
 * registered again — this promise rejects only when the combination is one Rust
 * will not offer at all (no modifier, a key it cannot draw, nonsense), or when
 * the ACL refused the call.
 *
 * The argument name is `accelerator` because Tauri matches a command's
 * parameters by name; renaming it here without renaming it in
 * `set_capture_shortcut` would fail at run time and nowhere else.
 */
export function setCaptureShortcut(accelerator: string): Promise<ShortcutChange> {
  return invoke<ShortcutChange>('set_capture_shortcut', { accelerator });
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
