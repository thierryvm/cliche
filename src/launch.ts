import { invoke } from '@tauri-apps/api/core';

/**
 * Starting a capture from the launcher, with the mouse.
 *
 * Mirrors `src-tauri/src/launch.rs`, which is where the reasoning lives: the
 * command runs the very `veil::perform_capture` the global shortcut runs, so
 * that the home screen's headline tile does the thing it is drawn as doing.
 */

/**
 * Asks the backend to start a region capture.
 *
 * # What resolving means, and what it does NOT
 *
 * Since 6 September 2026 it means a capture was SCHEDULED, which is weaker than
 * it was: the backend now hides this window, waits for Windows to repaint the
 * desktop without it, and only then photographs the screen - all on a thread of
 * its own, so this promise resolves before any of it has happened.
 * `src-tauri/src/launch.rs` has the reasoning and the figure that wait costs.
 *
 * It certainly does not mean a screenshot exists: the user still has to drag a
 * rectangle, and the capture ends later through the veil's own commands, out of
 * this promise's reach. Nothing that awaits this may say « copié ».
 *
 * Rejects with the backend's error string. The ACL and `ipc::ensure_from` both
 * refuse this command to any window but `main`, so a rejection here is a
 * misconfiguration rather than a failed capture - a failure inside the pipeline
 * abandons the run and prints, and never comes back this way.
 */
export function captureRegion(): Promise<void> {
  return invoke<void>('capture_region');
}
