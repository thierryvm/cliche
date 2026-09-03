/**
 * The veil page: show a frozen screen, say when it is painted, close on Escape.
 *
 * Bare TypeScript. No React, no design tokens, no component. The only import is
 * `invoke`, because the acknowledgement has to reach Rust and that is the
 * primitive that carries it.
 *
 * ## What `painted` really means
 *
 * The acknowledgement is sent from inside a `requestAnimationFrame` callback
 * taken AFTER `HTMLImageElement.decode()` resolves. That ordering is the whole
 * point: `decode()` resolves once the bitmap is ready to be drawn, so the
 * callback runs at a moment when the only thing left is the draw itself.
 *
 * It is still an approximation, and it is wrong in two directions at once:
 *
 * - **Too early.** A `requestAnimationFrame` callback runs BEFORE the compositor
 *   presents the frame. Nothing available to a page proves presentation, so
 *   this cannot be claimed as "the user saw it".
 * - **Too late.** Rust timestamps when the acknowledgement ARRIVES, so the
 *   figure carries the return trip of the message as well.
 *
 * Announcing it as a clean upper bound would be the flattering reading. It is
 * an estimate with one named over-count and one named under-count.
 *
 * Acknowledging on a single frame, rather than waiting for a second one, is a
 * deliberate choice: a double `requestAnimationFrame` would be closer to true
 * presentation but would add a whole frame of the page's own scheduling to
 * every measurement.
 */

import { invoke } from '@tauri-apps/api/core';

declare global {
  interface Window {
    /**
     * Called from Rust by `eval`, once the payload is ready.
     *
     * @param source Either `http://cliche.localhost/frame/<n>.bmp` (transport A)
     *   or a `data:image/png;base64,...` URL (transport B). Both are built in
     *   Rust from a fixed alphabet, so neither can carry a character that would
     *   break out of the string literal it travels in.
     * @param run The run number. Echoed back so that a stale acknowledgement -
     *   an image that finished decoding after the next capture started - can be
     *   told apart from a real one.
     */
    __clicheShow: (source: string, run: number) => void;
  }
}

const frame = document.getElementById('frame');

// Not a defensive nicety: without this node there is nothing to paint, and the
// failure would otherwise surface as an acknowledgement that never arrives -
// which reads, in the report, as a slow pipeline rather than a broken page.
if (!(frame instanceof HTMLImageElement)) {
  throw new Error('Cannot run the veil: #frame is missing from veil.html');
}

/**
 * The run currently being shown. An image whose decode finishes after a newer
 * run has started must not acknowledge: it would file a measurement under the
 * wrong run, and the timing instrument would record a `painted` for a run whose
 * `shown` belongs to a different capture.
 */
let currentRun = 0;

window.__clicheShow = (source: string, run: number): void => {
  currentRun = run;

  // Hidden first, so that the veil is black rather than showing the PREVIOUS
  // capture while the new one decodes. The window is made visible by Rust just
  // before this function is called, so this runs before the compositor has had
  // a chance to present anything stale - but "before" is a race, not a
  // guarantee, and a stale flash of one frame remains possible.
  frame.hidden = true;

  frame.src = source;

  frame
    .decode()
    .then(() => {
      if (run !== currentRun) {
        return;
      }

      frame.hidden = false;

      requestAnimationFrame(() => {
        if (run !== currentRun) {
          return;
        }
        // No `await`: the acknowledgement's own trip is already counted in the
        // figure, and waiting for its reply would add a second one.
        void invoke('veil_painted', { run }).catch((error: unknown) => {
          console.error('[cliche] veil: could not acknowledge the paint', error);
        });
      });
    })
    .catch((error: unknown) => {
      // A decode failure is the loudest symptom of a rejected CSP origin or a
      // malformed BMP header. It must not be swallowed: the run would then
      // simply never acknowledge, and a broken transport would look like a slow
      // one.
      console.error(`[cliche] veil: could not decode run ${run}`, error);
    });
};

window.addEventListener('keydown', (event: KeyboardEvent) => {
  if (event.key !== 'Escape') {
    return;
  }

  event.preventDefault();

  // Invalidated first: a decode still in flight must not acknowledge a run the
  // user has just cancelled. Without this line a cancelled capture could still
  // be filed as a successful measurement, which is exactly how a median gets
  // flattered.
  currentRun = 0;
  frame.hidden = true;
  frame.removeAttribute('src');

  void invoke('veil_dismissed').catch((error: unknown) => {
    console.error('[cliche] veil: could not dismiss', error);
  });
});
