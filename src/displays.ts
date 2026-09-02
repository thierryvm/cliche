import { invoke } from '@tauri-apps/api/core';

/**
 * One physical display, as reported by the Rust side.
 *
 * Sizes are PHYSICAL pixels, not logical ones: `width`/`height` are the raw
 * pixel grid of the panel, unaffected by the Windows scaling setting. Dividing
 * by `scaleFactor` gives the logical size the webview thinks in. Screenshot
 * geometry must be computed in physical pixels, which is why this is the unit
 * crossing the IPC boundary.
 *
 * Field names mirror `DisplayInfo` in `src-tauri/src/displays.rs`, which
 * serialises with `rename_all = "camelCase"`.
 */
export interface DisplayInfo {
  /** OS name of the monitor, e.g. `\\.\DISPLAY1`. Empty when Windows reports none. */
  readonly name: string;
  /** X of the top-left corner in the virtual desktop, physical pixels. May be negative. */
  readonly x: number;
  /** Y of the top-left corner in the virtual desktop, physical pixels. May be negative. */
  readonly y: number;
  /** Width in physical pixels. */
  readonly width: number;
  /** Height in physical pixels. */
  readonly height: number;
  /** Logical-to-physical ratio: 1.0 at 100 % scaling, 1.5 at 150 %, etc. */
  readonly scaleFactor: number;
}

/**
 * Asks the backend to enumerate every display. The backend also writes the same
 * information to stdout, so a run of `pnpm tauri dev` leaves a trace in the
 * terminal even if the window fails to render.
 *
 * Rejects with the backend's error string when the monitor list cannot be read.
 */
export function describeDisplays(): Promise<DisplayInfo[]> {
  return invoke<DisplayInfo[]>('describe_displays');
}
