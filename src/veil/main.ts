/**
 * The veil page: show a frozen screen, say when it is painted, let the user
 * draw a rectangle on it, close on Escape.
 *
 * Bare TypeScript. No React, no component layer. The only import is `invoke`,
 * because both the paint acknowledgement and the selection have to reach Rust
 * and that is the primitive that carries them. The design tokens ARE loaded, as
 * a stylesheet linked from veil.html - see the comment there for why that costs
 * the budget nothing.
 *
 * ## This file performs NO coordinate conversion
 *
 * Worth stating first, because it is the thing most likely to be undone by a
 * later well-meant edit. The rectangle travels to Rust as the two corners the
 * pointer reported, in CSS pixels, unmultiplied. There is no
 * `devicePixelRatio` in this file and there must not be one:
 * `src-tauri/src/geometry.rs` is the single place where CSS pixels become
 * physical pixels, and the single place where the rounding rule is decided -
 * because that is a place with tests that can run at 125 %, and a pointer
 * handler is not.
 *
 * The only arithmetic here is clamping to the viewport and taking min/abs to
 * DRAW the rectangle. Rust normalises the same two corners with the same rule,
 * so what is shown and what is cut come from one decision rather than two.
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
const selection = document.getElementById('selection');

// Not a defensive nicety: without these nodes there is nothing to paint and
// nothing to draw on, and the failure would otherwise surface as an
// acknowledgement that never arrives - which reads, in the report, as a slow
// pipeline rather than a broken page.
if (!(frame instanceof HTMLImageElement)) {
  throw new Error('Cannot run the veil: #frame is missing from veil.html');
}
if (!(selection instanceof HTMLElement)) {
  throw new Error('Cannot run the veil: #selection is missing from veil.html');
}

/**
 * The run currently being shown. An image whose decode finishes after a newer
 * run has started must not acknowledge: it would file a measurement under the
 * wrong run, and the timing instrument would record a `painted` for a run whose
 * `shown` belongs to a different capture.
 *
 * Zero means "nothing is on screen", which is also what stops a pointer press
 * on a closed veil from drawing anything.
 */
let currentRun = 0;

/** Where the drag started, in CSS pixels. `null` when no drag is in progress. */
let anchor: { x: number; y: number } | null = null;

/** The pointer that owns the drag, so a second finger cannot hijack it. */
let dragPointer: number | null = null;

/**
 * Clears everything the page is showing and forgets the run in flight.
 *
 * `currentRun` is invalidated FIRST: a decode still in flight must not
 * acknowledge a run the user has just finished with. Without that line a
 * cancelled or completed capture could still be filed as a successful
 * measurement, which is exactly how a median gets flattered.
 */
const reset = (): void => {
  currentRun = 0;
  anchor = null;
  dragPointer = null;
  selection.hidden = true;
  frame.hidden = true;
  frame.removeAttribute('src');
};

/**
 * Keeps a pointer coordinate inside the veil document.
 *
 * The clamp is HERE and not in Rust, because this is the only side that knows
 * how big the veil is. Rust REFUSES a negative coordinate rather than repairing
 * one: a coordinate outside this document would mean the page sent a number it
 * never measured, and quietly fixing that is how a wrong rectangle becomes a
 * wrong screenshot nobody notices.
 *
 * A pointer can leave this window during a drag - the veil covers one monitor,
 * not the desktop - so the clamp is reached in ordinary use, not only in
 * failure.
 */
const clamp = (value: number, limit: number): number =>
  Math.min(Math.max(value, 0), limit);

/** The pointer position, clamped to the viewport. */
const at = (event: PointerEvent): { x: number; y: number } => ({
  x: clamp(event.clientX, window.innerWidth),
  y: clamp(event.clientY, window.innerHeight),
});

/**
 * Lays the selection rectangle over the two corners.
 *
 * Written straight to the style, not deferred to a `requestAnimationFrame`:
 * the browser already coalesces pointer moves, and a frame of deferral would be
 * a frame of lag between the hand and the rectangle. The selection is drawn
 * after the veil is painted, so none of this is inside the 150 ms budget.
 */
const drawSelection = (
  from: { x: number; y: number },
  to: { x: number; y: number },
): void => {
  selection.style.left = `${Math.min(from.x, to.x)}px`;
  selection.style.top = `${Math.min(from.y, to.y)}px`;
  selection.style.width = `${Math.abs(to.x - from.x)}px`;
  selection.style.height = `${Math.abs(to.y - from.y)}px`;
  selection.hidden = false;
};

window.__clicheShow = (source: string, run: number): void => {
  currentRun = run;
  // A rectangle left over from the previous capture must not appear over the
  // new one, even for a frame.
  anchor = null;
  dragPointer = null;
  selection.hidden = true;

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

window.addEventListener('pointerdown', (event: PointerEvent) => {
  // The primary button only: a right-click is not a selection. And nothing is
  // drawn when no capture is on screen, which is what `currentRun === 0` means.
  if (event.button !== 0 || currentRun === 0 || anchor !== null) {
    return;
  }

  // Stops WebView2 starting a native drag or a text selection under the hand.
  event.preventDefault();

  anchor = at(event);
  dragPointer = event.pointerId;
  drawSelection(anchor, anchor);

  // The pointer may leave this window mid-drag - onto a second monitor, or off
  // the edge of the screen. Without capture the moves and the release would go
  // elsewhere and the rectangle would freeze mid-gesture. Capture is an
  // improvement, not a requirement, so a refusal is reported and the drag
  // continues.
  try {
    document.documentElement.setPointerCapture(event.pointerId);
  } catch (error: unknown) {
    console.error('[cliche] veil: could not capture the pointer', error);
  }
});

window.addEventListener('pointermove', (event: PointerEvent) => {
  if (anchor === null || event.pointerId !== dragPointer) {
    return;
  }

  event.preventDefault();
  drawSelection(anchor, at(event));
});

window.addEventListener('pointerup', (event: PointerEvent) => {
  if (anchor === null || event.pointerId !== dragPointer) {
    return;
  }

  const from = anchor;
  const to = at(event);
  const run = currentRun;

  anchor = null;
  dragPointer = null;
  if (document.documentElement.hasPointerCapture(event.pointerId)) {
    document.documentElement.releasePointerCapture(event.pointerId);
  }

  // A click is not a drag. Under one CSS pixel in either axis there is no
  // rectangle to cut - at 125 % it would not even cover a whole physical pixel,
  // and Rust would refuse it. The page forgets the gesture rather than sending
  // an error round trip for a stray click on the veil.
  if (Math.abs(to.x - from.x) < 1 || Math.abs(to.y - from.y) < 1) {
    selection.hidden = true;
    return;
  }

  // The rectangle stops being drawn as soon as the hand lifts, but the frozen
  // image stays: if Rust refuses the selection, the veil is still usable.
  selection.hidden = true;

  // The four numbers go over as measured. No scale, no rounding - see the file
  // header.
  void invoke('veil_selected', {
    run,
    x0: from.x,
    y0: from.y,
    x1: to.x,
    y1: to.y,
  })
    .then(() => {
      // Only if a newer capture has not started in the meantime. Same reasoning
      // as the decode acknowledgement above: a reply belonging to run 3 must
      // not tear down run 4.
      if (currentRun === run) {
        reset();
      }
    })
    .catch((error: unknown) => {
      // Deliberately NOT dismissed. The frozen image stays on screen so the
      // user can draw again or press Escape; a veil that vanished on an error
      // would be indistinguishable from a capture that worked.
      console.error('[cliche] veil: the selection was refused', error);
    });
});

window.addEventListener('pointercancel', (event: PointerEvent) => {
  if (event.pointerId !== dragPointer) {
    return;
  }

  // The system took the pointer away - a gesture, a touch cancelled. The drag
  // is abandoned; the veil stays open because the user never asked to close it.
  anchor = null;
  dragPointer = null;
  selection.hidden = true;
});

window.addEventListener('keydown', (event: KeyboardEvent) => {
  if (event.key !== 'Escape') {
    return;
  }

  event.preventDefault();

  reset();

  void invoke('veil_dismissed').catch((error: unknown) => {
    console.error('[cliche] veil: could not dismiss', error);
  });
});
