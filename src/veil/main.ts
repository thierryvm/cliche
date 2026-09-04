/**
 * The veil page: show a frozen screen, say when it is painted, let the user
 * draw a rectangle on it, resize it, move it, copy it, close on Escape.
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
 * ## THE SELECTION IS TWO ABSOLUTE CORNERS, AND NEVER A SIZE
 *
 * The rule that makes resizing safe, and the one to read before touching
 * `resize` or `move` below. The state is `anchor` and `pointer`: two points, in
 * the coordinates the pointer reported, absolute. No width is stored, no delta
 * is ever added to anything.
 *
 * The consequence is the property Thierry asked for on 4 September 2026 - "la
 * decoupe au pixel doit rester exacte apres un redimensionnement" - and it is
 * under test in Rust, in `capture.rs`
 * (`a_sequence_of_resizes_cuts_the_same_bytes_as_the_rectangle_drawn_directly`,
 * with the accumulating implementation next to it to prove that test can fail):
 * a rectangle reached by any number of resizes cuts the same bytes as the same
 * rectangle drawn in one gesture. An implementation that carried a size and
 * added each gesture's movement to it would round once per gesture and lose
 * pixels that no test in the Rust half could see.
 *
 * A resize is therefore not a second kind of gesture. It is a DRAW whose anchor
 * is the opposite corner, which is also why nothing here handles inversion:
 * drag a corner past its opposite and `drawSelection` and Rust's
 * `CssRect::from_corners` both normalise, with no branch on either side.
 *
 * ## The page sends TWO acknowledgements, and only one of them shows the veil
 *
 * Changed on 4 September 2026. `veil_decoded` goes out the moment
 * `HTMLImageElement.decode()` resolves, and it is what tells Rust to make the
 * window visible - until it arrives the veil is a hidden window that has already
 * finished its work. `veil_painted` follows one animation frame later and closes
 * the measured run.
 *
 * The reason for the split is that the window used to be shown FIRST, so it
 * spent the whole decode - median 91.3 ms, p95 94.3 ms over 18 clean runs on
 * 868ba0d, measured 4 September 2026 - displaying the previous capture. That is
 * what was seen as flashing.
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
 *
 * ## Nothing added by this lot is inside that measurement
 *
 * The grips, the keyboard line and the refusal plate are all static markup
 * wearing `hidden` since the preheat, and none of them is unhidden before the
 * first pointer press - which is after `painted`, at human speed. None of them
 * carries `backdrop-filter`, `transform`, `will-change` or an opacity below 1,
 * each of which would make the compositor build a layer AT PARSE, inside the
 * interval lot 1d measures. The p95 leaves 15.7 ms of margin; this lot spends
 * none of it. That is reasoning about how Blink is understood to work, not a
 * measurement taken on this machine - the same standing caveat as `#edge`.
 *
 * ## The edge frame is inside that acknowledgement, and this is why
 *
 * veil.html draws a 4 px two-tone band round the screen, so that a veil which
 * is a pixel-exact copy of the desktop can still be told from the desktop. A
 * decoration that appeared AFTER the acknowledgement would be worse than no
 * decoration: its cost would fall outside the figure, and the next measurement
 * would under-report by exactly the thing that was added.
 *
 * It cannot. The band is static markup (`#edge`) wearing static CSS, in the
 * document since the window was preheated, touched by no code in this file. Any
 * frame the compositor builds after parse therefore contains it, including the
 * one this acknowledgement is scheduled inside.
 *
 * What is REASONED rather than measured is the cost, and it is worth naming
 * precisely. `#edge` triggers nothing that would promote it to a compositor
 * layer of its own - no transform, no opacity, no will-change - so it rasterises
 * with everything else in the root layer, and unhiding the image invalidates the
 * tiles the band lives in too. That is how Blink is understood to work; it is
 * not a reading taken on this machine. NOBODY HAS MEASURED THIS BAND. If the
 * band ever did end up on its own layer, its raster would happen once at preheat
 * and the 150 ms figure would silently stop containing it - the failure would be
 * a flattering number, not a visible bug, which is the kind that survives.
 */

import { invoke } from '@tauri-apps/api/core';

// THE ZONE MODEL lives in `./zones`, and the split is structural rather than
// tidiness: this file queries the DOM at module load, so a Node test process
// cannot import it. `zones.ts` touches nothing but its arguments, which is what
// puts `hitTest` and the anchor rule under test with no simulated DOM. Read the
// header there before moving anything back.
import {
  cursorForZone,
  grabFor,
  hitTest,
  lengthInPixels,
  movingCorner,
} from './zones';
import type { CursorName, Grab, Point, Rect } from './zones';

declare global {
  interface Window {
    /**
     * Called from Rust by `eval`, once the payload is ready, ON A WINDOW THAT
     * IS STILL HIDDEN. It is this function's `veil_decoded` call that makes the
     * window appear.
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

// ---------------------------------------------------------------------------
// THE PAGE.
// ---------------------------------------------------------------------------

const frame = document.getElementById('frame');
const selection = document.getElementById('selection');
const edge = document.getElementById('edge');
const hint = document.getElementById('hint');
const notice = document.getElementById('notice');

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
// #edge is read here and nowhere else, and that is the point. It is the band
// that tells the user the veil is open at all; deleted by a later edit it would
// take no test and no error with it, and the defect of 4 September 2026 - a
// veil indistinguishable from the desktop it covers - would simply come back in
// silence. This repo has no DOM test runner, so a throw at load is the only
// assertion available. It fires during the preheat, before any shortcut, and
// therefore outside the 150 ms budget.
if (!(edge instanceof HTMLElement)) {
  throw new Error('Cannot run the veil: #edge is missing from veil.html');
}
// Same reasoning, and the same moment. A veil that lost its keyboard line would
// stop telling anyone that Enter copies - and Enter is now the only validation
// that is guaranteed to exist, the double-click being a convenience.
if (!(hint instanceof HTMLElement)) {
  throw new Error('Cannot run the veil: #hint is missing from veil.html');
}
if (!(notice instanceof HTMLElement)) {
  throw new Error('Cannot run the veil: #notice is missing from veil.html');
}

const root = document.documentElement;

/**
 * The two geometry tokens, resolved once during the preheat.
 *
 * Read from CSS rather than typed here: `src/design/tokens.css` forbids a
 * second list of its values in TypeScript, and a hit-testing distance that
 * disagreed with the dot drawn on the screen is exactly the kind of drift that
 * rule exists for. The cost is one style read, seconds before any shortcut.
 */
const tokenPixels = (name: string): number => {
  const computed = getComputedStyle(root);
  const rootFontSize = Number.parseFloat(computed.fontSize);
  const raw = computed.getPropertyValue(name);

  try {
    return lengthInPixels(raw, rootFontSize);
  } catch (error: unknown) {
    throw new Error(
      `Cannot run the veil: ${name} does not resolve to a length (${String(error)})`,
    );
  }
};

/** Width of the ring of grab zones, outside the rectangle. */
const GRIP_OUTSET = tokenPixels('--veil-grip-outset');

/** The shortest side that still has room for a midpoint dot. */
const GRIP_ROOM = tokenPixels('--veil-grip-room');

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

/**
 * THE SELECTION: two absolute corners, or nothing. Read the header before
 * replacing this with an origin and a size.
 *
 * It now OUTLIVES the drag that made it - that is the whole of this lot. The
 * rectangle stays on screen after the hand lifts, wearing its grips, until
 * Enter or a double-click copies it or Escape throws it away.
 */
let corners: { anchor: Point; pointer: Point } | null = null;

/** What is happening under the hand right now, if anything. */
type Gesture =
  | { readonly kind: 'corner'; readonly grab: Grab }
  | { readonly kind: 'move'; readonly hold: Point; readonly size: Point };

let gesture: Gesture | null = null;

/** The pointer that owns the gesture, so a second finger cannot hijack it. */
let gesturePointer: number | null = null;

/**
 * The geometry as it was before the gesture started.
 *
 * `pointercancel` restores it. It used to hide the selection instead, which is
 * right for a drag that was drawing a new rectangle and wrong for every other
 * case: the system taking the pointer away in the middle of a resize would have
 * thrown away a rectangle the user had already drawn and never asked to lose.
 */
let before: { anchor: Point; pointer: Point } | null = null;

/** The cursor currently written on the root, so it is written only on change. */
let cursor: CursorName = null;

const setCursor = (next: CursorName): void => {
  if (next === cursor) {
    return;
  }
  cursor = next;
  if (next === null) {
    root.removeAttribute('data-veil-cursor');
  } else {
    root.setAttribute('data-veil-cursor', next);
  }
};

/** The four edges of the current selection, normalised. */
const rectOf = (pair: { anchor: Point; pointer: Point }): Rect => ({
  left: Math.min(pair.anchor.x, pair.pointer.x),
  top: Math.min(pair.anchor.y, pair.pointer.y),
  right: Math.max(pair.anchor.x, pair.pointer.x),
  bottom: Math.max(pair.anchor.y, pair.pointer.y),
});

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
  Math.min(Math.max(value, 0), Math.max(limit, 0));

/** The pointer position, clamped to the viewport. */
const at = (event: PointerEvent): Point => ({
  x: clamp(event.clientX, window.innerWidth),
  y: clamp(event.clientY, window.innerHeight),
});

/**
 * Lays the selection rectangle over its two corners, and says which midpoint
 * dots have room to be drawn.
 *
 * Written straight to the style, not deferred to a `requestAnimationFrame`:
 * the browser already coalesces pointer moves, and a frame of deferral would be
 * a frame of lag between the hand and the rectangle. The selection is drawn
 * after the veil is painted, so none of this is inside the 150 ms budget.
 */
const drawSelection = (pair: { anchor: Point; pointer: Point }): void => {
  const rect = rectOf(pair);
  const width = rect.right - rect.left;
  const height = rect.bottom - rect.top;

  selection.style.left = `${rect.left}px`;
  selection.style.top = `${rect.top}px`;
  selection.style.width = `${width}px`;
  selection.style.height = `${height}px`;

  // Only about what is DRAWN. The zones stay whole either way - see the rule in
  // veil.html. `toggle` with a force argument writes nothing when the class is
  // already in the state asked for, so a move that does not cross the threshold
  // costs no style invalidation.
  selection.classList.toggle('has-width', width >= GRIP_ROOM);
  selection.classList.toggle('has-height', height >= GRIP_ROOM);

  selection.hidden = false;
};

/** Puts the refusal away. Called as soon as the user acts again. */
const clearNotice = (): void => {
  if (!notice.hidden) {
    notice.hidden = true;
    notice.textContent = '';
  }
};

/**
 * Hands the pointer back, if this page ever took it.
 *
 * `hasPointerCapture` first: releasing a capture that was never taken throws,
 * and every caller of this is on a path that must not.
 */
const releaseCapture = (): void => {
  if (gesturePointer === null) {
    return;
  }
  if (root.hasPointerCapture(gesturePointer)) {
    root.releasePointerCapture(gesturePointer);
  }
};

/**
 * Clears everything the page is showing and forgets the run in flight.
 *
 * `currentRun` is invalidated FIRST: a decode still in flight must not
 * acknowledge a run the user has just finished with. Without that line a
 * cancelled or completed capture could still be filed as a successful
 * measurement, which is exactly how a median gets flattered.
 *
 * IT RELEASES THE POINTER CAPTURE, and that line is a repair. Escape pressed
 * with the button still down used to leave `documentElement` owning the
 * pointer: the veil was gone, the capture was not, and the next window to see
 * that pointer was not the one under it.
 */
const reset = (): void => {
  currentRun = 0;
  corners = null;
  gesture = null;
  before = null;
  releaseCapture();
  gesturePointer = null;
  setCursor(null);
  selection.hidden = true;
  hint.hidden = true;
  clearNotice();
  frame.hidden = true;
  frame.removeAttribute('src');
};

window.__clicheShow = (source: string, run: number): void => {
  // A rectangle, a hint or a refusal left over from the previous capture must
  // not appear over the new one, even for a frame. `reset` cannot be used here:
  // it clears `currentRun`, which is set immediately below.
  corners = null;
  gesture = null;
  before = null;
  releaseCapture();
  gesturePointer = null;
  setCursor(null);
  selection.hidden = true;
  hint.hidden = true;
  clearNotice();

  currentRun = run;

  // Hidden first, and this line now carries far more weight than it did.
  //
  // Since 4 September 2026 the window is still HIDDEN when this function runs;
  // Rust shows it on `veil_decoded` below. So on the ordinary path this line
  // guards nothing visible - but on the fallback path (`veil::arm_show_fallback`,
  // 250 ms) the window is shown while this decode is still in flight, and this
  // is what makes the user see a BLACK veil filling in rather than the previous
  // capture. Deleting it would turn the fallback from a repair into a leak of
  // the last screenshot.
  frame.hidden = true;

  frame.src = source;

  frame
    .decode()
    .then(() => {
      if (run !== currentRun) {
        return;
      }

      // Unhidden BEFORE Rust is told, so that the very first frame the
      // compositor builds for the newly visible window already contains the
      // image. The style write happens here; the window appears after a round
      // trip; there is no ordering in which the window is visible and this
      // element is not.
      frame.hidden = false;

      // THE CALL THAT MAKES THE VEIL APPEAR. Rust holds the window hidden from
      // the shortcut until this arrives - see the ORDER comment in
      // `perform_capture` for the objection this design routes around.
      //
      // WHY THAT IS SAFE, and it is the property to keep in mind before moving
      // anything below back above this line: the risky part of a hidden webview
      // is `requestAnimationFrame`, which WebView2 throttles when the window is
      // not visible. There is no rAF above this call. The one below runs on a
      // window Rust has been asked to show - and if it is nonetheless throttled
      // for the fraction of a second the show takes, the ONLY casualty is the
      // `painted` measurement. The veil still appears. The danger was moved off
      // the critical path and onto the instrument.
      //
      // No `await`: nothing here needs the reply, and waiting for one would add
      // a round trip to the interval Rust is timing.
      void invoke('veil_decoded', { run }).catch((error: unknown) => {
        console.error('[cliche] veil: could not report the decode', error);
      });

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
      //
      // It is now also the case that never acknowledging means the veil never
      // appears on its own. What the user gets instead is the fallback, 250 ms
      // later, and a line in the terminal - not silence.
      console.error(`[cliche] veil: could not decode run ${run}`, error);
    });
};

/**
 * Sends the rectangle to Rust to be cut and copied.
 *
 * Reached by Enter and by a double-click inside the selection, and by nothing
 * else. Lifting the hand no longer copies: a gesture that copied on release
 * could not also be the gesture that begins a resize.
 */
const commit = (): void => {
  if (corners === null || currentRun === 0) {
    return;
  }

  const rect = rectOf(corners);
  const run = currentRun;

  clearNotice();

  // The four numbers go over as measured. No scale, no rounding - see the file
  // header.
  void invoke('veil_selected', {
    run,
    x0: rect.left,
    y0: rect.top,
    x1: rect.right,
    y1: rect.bottom,
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
      // SHOWN, not merely logged - and that is a repair. Rust refuses a
      // selection below `MIN_COPYABLE_AREA_PX` with a sentence that names the
      // size, the area and the threshold (`clipboard::too_small_line`, under
      // test there). It used to go to `console.error` alone: with Enter as the
      // validation, a refusal nobody displays is a screen that does not answer
      // a key.
      //
      // The selection stays exactly as it was, and stays editable: the fix for
      // "too small" is to make it bigger, which is now a gesture away.
      console.error('[cliche] veil: the selection was refused', error);
      if (currentRun !== run) {
        return;
      }
      notice.textContent =
        typeof error === 'string' ? error : `the selection was refused: ${String(error)}`;
      notice.hidden = false;
    });
};

window.addEventListener('pointerdown', (event: PointerEvent) => {
  // The primary button only: a right-click is not a selection. And nothing is
  // drawn when no capture is on screen, which is what `currentRun === 0` means.
  if (event.button !== 0 || currentRun === 0 || gesture !== null) {
    return;
  }

  // Stops WebView2 starting a native drag or a text selection under the hand.
  event.preventDefault();

  clearNotice();

  const point = at(event);
  const zone =
    corners === null ? 'outside' : hitTest(rectOf(corners), point, GRIP_OUTSET);

  before = corners;

  if (corners !== null && zone === 'inside') {
    const rect = rectOf(corners);
    // The offset from the rectangle's ORIGIN to the hand, frozen at the press.
    // What gets clamped further down is that origin, never the pointer: clamp
    // the pointer and the far edge walks off the screen while the near one
    // stays put.
    gesture = {
      kind: 'move',
      hold: { x: point.x - rect.left, y: point.y - rect.top },
      size: { x: rect.right - rect.left, y: rect.bottom - rect.top },
    };
    setCursor('move');
  } else {
    const grab = corners === null ? null : grabFor(zone, rectOf(corners));
    if (grab === null) {
      // A fresh rectangle: a draw is a resize whose anchor is where the hand
      // went down. One code path, not two.
      gesture = {
        kind: 'corner',
        grab: { anchor: point, followX: true, followY: true, pinned: point },
      };
      corners = { anchor: point, pointer: point };
    } else {
      gesture = { kind: 'corner', grab };
      corners = { anchor: grab.anchor, pointer: movingCorner(grab, point) };
    }
    setCursor(cursorForZone(zone));
  }

  gesturePointer = event.pointerId;
  drawSelection(corners);

  // The pointer may leave this window mid-drag - onto a second monitor, or off
  // the edge of the screen. Without capture the moves and the release would go
  // elsewhere and the rectangle would freeze mid-gesture. Capture is an
  // improvement, not a requirement, so a refusal is reported and the drag
  // continues.
  try {
    root.setPointerCapture(event.pointerId);
  } catch (error: unknown) {
    console.error('[cliche] veil: could not capture the pointer', error);
  }
});

window.addEventListener('pointermove', (event: PointerEvent) => {
  if (gesture === null || gesturePointer === null) {
    // No gesture: the cursor follows the zone under the hand, so the ring can
    // be found before it is grabbed.
    if (corners !== null && currentRun !== 0) {
      setCursor(cursorForZone(hitTest(rectOf(corners), at(event), GRIP_OUTSET)));
    }
    return;
  }

  if (event.pointerId !== gesturePointer) {
    return;
  }

  event.preventDefault();
  const point = at(event);

  if (gesture.kind === 'move') {
    // The ORIGIN is clamped, not the pointer. `Math.max(limit, 0)` inside
    // `clamp` covers a selection wider than the viewport, which a resize can
    // produce at the very edge.
    const left = clamp(point.x - gesture.hold.x, window.innerWidth - gesture.size.x);
    const top = clamp(point.y - gesture.hold.y, window.innerHeight - gesture.size.y);
    corners = {
      anchor: { x: left, y: top },
      pointer: { x: left + gesture.size.x, y: top + gesture.size.y },
    };
    // Held to the gesture: leaving the interior mid-move must not turn the
    // cursor into a resize arrow.
    setCursor('move');
  } else {
    const { grab } = gesture;
    const moving = movingCorner(grab, point);
    corners = { anchor: grab.anchor, pointer: moving };

    // Recomputed from the LIVE geometry on every frame, never frozen at the
    // press: drag a corner past its opposite one and the diagonal really has
    // swapped, so nwse must become nesw under the hand.
    if (grab.followX && grab.followY) {
      const rightOfAnchor = moving.x >= grab.anchor.x;
      const belowAnchor = moving.y >= grab.anchor.y;
      setCursor(rightOfAnchor === belowAnchor ? 'nwse' : 'nesw');
    } else {
      setCursor(grab.followX ? 'ew' : 'ns');
    }
  }

  drawSelection(corners);
});

window.addEventListener('pointerup', (event: PointerEvent) => {
  if (gesture === null || event.pointerId !== gesturePointer) {
    return;
  }

  gesture = null;
  releaseCapture();
  gesturePointer = null;

  // A click is not a drag. Under one CSS pixel in either axis there is no
  // rectangle to cut - at 125 % it would not even cover a whole physical pixel,
  // and Rust would refuse it. What was on screen before the press comes back:
  // a stray click on a rectangle the user has already drawn must not destroy
  // it, and before the first drag there is nothing to come back to.
  if (corners !== null) {
    const rect = rectOf(corners);
    if (rect.right - rect.left < 1 || rect.bottom - rect.top < 1) {
      corners = before;
      if (corners === null) {
        selection.hidden = true;
      } else {
        drawSelection(corners);
      }
    }
  }

  before = null;
  setCursor(
    corners === null
      ? null
      : cursorForZone(hitTest(rectOf(corners), at(event), GRIP_OUTSET)),
  );

  // The keyboard line, revealed the first time a hand lifts and not before.
  // Everything it names is available from this moment: there is a rectangle to
  // copy. It costs nothing at opening - see the note in veil.html.
  if (corners !== null) {
    hint.hidden = false;
  }
});

/**
 * A double-click inside the selection copies it.
 *
 * A CONVENIENCE, never the only route: Enter does the same thing and is the one
 * that is guaranteed to reach here. `pointerdown` calls `preventDefault`, which
 * suppresses the compatibility mouse events; `click` and `dblclick` are
 * documented as unaffected by that, but this has NOT been verified in WebView2
 * on this machine - the veil cannot be opened without taking over the screen of
 * whoever is working on it.
 */
window.addEventListener('dblclick', (event: MouseEvent) => {
  if (corners === null || currentRun === 0) {
    return;
  }

  const point = {
    x: clamp(event.clientX, window.innerWidth),
    y: clamp(event.clientY, window.innerHeight),
  };
  if (hitTest(rectOf(corners), point, GRIP_OUTSET) !== 'inside') {
    return;
  }

  event.preventDefault();
  commit();
});

window.addEventListener('pointercancel', (event: PointerEvent) => {
  if (event.pointerId !== gesturePointer) {
    return;
  }

  // The system took the pointer away - a gesture, a touch cancelled. The veil
  // stays open because the user never asked to close it, and the geometry goes
  // back to what it was before this gesture rather than being thrown away:
  // during a resize or a move there is a rectangle to lose.
  gesture = null;
  releaseCapture();
  gesturePointer = null;
  corners = before;
  before = null;
  setCursor(null);

  if (corners === null) {
    selection.hidden = true;
  } else {
    drawSelection(corners);
  }
});

window.addEventListener('keydown', (event: KeyboardEvent) => {
  if (event.key === 'Enter') {
    if (corners === null || currentRun === 0) {
      return;
    }
    event.preventDefault();
    commit();
    return;
  }

  if (event.key !== 'Escape') {
    return;
  }

  event.preventDefault();

  // ONE MEANING FOR THIS KEY: cancel the capture. Including in the middle of a
  // drag, where it used to close the veil while leaving the pointer captured -
  // `reset` now ends the gesture properly, which is the actual defect.
  //
  // Giving Escape a second meaning - "abandon this gesture, keep the veil" -
  // was considered and refused: the user would have to know which of the two
  // states they are in before pressing a key whose whole value is that it
  // always does the same thing.
  reset();

  void invoke('veil_dismissed').catch((error: unknown) => {
    console.error('[cliche] veil: could not dismiss', error);
  });
});
