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
 * It means a capture was STARTED - the screen is frozen and the veil has been
 * handed the frame. It does not mean a screenshot exists: the user still has to
 * drag a rectangle, and the capture ends later through the veil's own commands,
 * out of this promise's reach. Nothing that awaits this may say « copié ».
 *
 * Rejects with the backend's error string. The ACL and `ipc::ensure_from` both
 * refuse this command to any window but `main`, so a rejection here is a
 * misconfiguration rather than a failed capture - a failure inside the pipeline
 * abandons the run and prints, and never comes back this way.
 */
export function captureRegion(): Promise<void> {
  return invoke<void>('capture_region');
}
