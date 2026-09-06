/**
 * The veil page: show a frozen screen, say when it is painted, let the user
 * draw a rectangle on it, resize it, move it, copy it, SAY WHETHER THE COPY
 * WORKED, close on Escape.
 *
 * Bare TypeScript. No React and no component runtime. `invoke` is imported
 * because every acknowledgement has to reach Rust and that is the primitive
 * that carries them; the string catalogue and the two pure modules next door
 * are the rest. The design tokens AND the component stylesheet are loaded, as
 * two links in veil.html - see the comment there for why that costs the budget
 * nothing, and for what the component layer does not bring in with it.
 *
 * ## What happens after a copy - 5 September 2026
 *
 * Until that day the window was hidden the instant the clipboard took the
 * image, and a clipboard that REFUSED wrote a sentence into a plate this file
 * drew by hand. Both are gone. A copy that worked leaves the veil up,
 * transparent and no longer answering the pointer, showing the system's own
 * `c-note c-note--success c-toast` for the length of `--dur-toast-dwell`; a
 * copy that did not happen leaves it opaque and interactive, showing
 * `c-note c-note--danger c-toast`, until the message is dismissed or Escape is
 * pressed. WHICH of the two, with which ARIA role, for how long, and whether
 * this window still answers the pointer, is decided in `./confirmation` and
 * mirrored in Rust by `veil::AfterSelection`.
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
 * A THIRD message leaves this page since the cold-start tracer was added below,
 * and it is not an acknowledgement: `veil_ready` reports which phase this script
 * has reached. Its `loaded` call site runs during the preheat, seconds before
 * any shortcut. Its `show-entered` call site IS inside the `decoded` step, and
 * fires ONCE per page - on the first capture after launch, the run the fallback
 * already discards. Runs 2 onward therefore travel the exact path a134cb7
 * measured. See `reportPhase`, and `veil::veil_ready` in Rust.
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
 * The grips, the keyboard line and the toast are all static markup wearing
 * `hidden` since the preheat, and none of them is unhidden before the first
 * pointer press - which is after `painted`, at human speed. None of them
 * carries `backdrop-filter`, `transform`, `will-change` or an opacity below 1
 * AT PARSE, each of which would make the compositor build a layer inside the
 * interval lot 1d measures. `.c-toast` does declare an `animation` whose
 * keyframes move `transform` and `opacity` - but its region is `hidden`, so the
 * whole subtree is out of the render tree until a selection has been judged,
 * and an element that is not rendered gets no layer. The p95 leaves 15.7 ms of
 * margin; this lot spends none of it. That is reasoning about how Blink is
 * understood to work, not a measurement taken on this machine - the same
 * standing caveat as `#edge`.
 *
 * ONE THING IN THIS LOT IS NOT COVERED BY THAT ARGUMENT, and it is the window
 * rather than the page: `create` now builds the veil TRANSPARENT, which changes
 * how Windows composes it for its whole life and not only while the
 * confirmation is up. The `painted` figures quoted below were taken on an
 * opaque window. They must be re-measured.
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

// THE SAME SPLIT, for the same reason, applied to what happens once the
// clipboard has answered: `./confirmation` decides WHICH message, with which
// ARIA role, for how long, and whether this window still answers the pointer.
// Read the header there before moving any of it back here - in particular the
// asymmetry between a copy that worked and one that did not.
import { millisecondsIn, planFor, sizeLabel } from './confirmation';
import type { ToastPlan, ToastTiming } from './confirmation';

// The wording, from the ONE catalogue. `scripts/check-strings.mjs` fails the
// suite the day a label below is re-typed here instead of imported, and the day
// the catalogue stops agreeing with `src/design/Showcase.tsx`.
import { UI_STRINGS } from '../strings';

// ---------------------------------------------------------------------------
// THE COLD-START TRACER. Added 4 September 2026, and it is an instrument.
// ---------------------------------------------------------------------------

/**
 * Reports which phase this script has reached.
 *
 * WHAT IT IS FOR. The first capture after a launch does not acknowledge: on
 * 868ba0d runs 1 and 2 of the benchmark never did within the bench's 3 s, and on
 * a134cb7 run 1 alone - rescued 250 ms later by `veil::arm_show_fallback`, which
 * is a delay the user meets at every cold start. Three explanations fit that
 * observation equally well, and the terminal cannot currently tell them apart:
 * the script of a never-shown window not running at all, `window.eval` not
 * reaching such a window, or a first `decode()` simply taking longer than 3 s.
 * `veil::veil_ready` has the table that reads the lines.
 *
 * The union type is the closed list of `veil::READY_PHASES` written a second
 * time. Rust validates the string anyway - it arrives over IPC and is printed on
 * a terminal - but a typo here should be a type error, not a refusal discovered
 * while reading a report.
 *
 * No `await`, like every other acknowledgement in this file: nothing needs the
 * reply, and waiting for one would put a round trip where a post belongs.
 */
const reportPhase = (phase: 'loaded' | 'show-entered'): void => {
  void invoke('veil_ready', { phase }).catch((error: unknown) => {
    console.error(`[cliche] veil: could not report the phase ${phase}`, error);
  });
};

/**
 * Whether `show-entered` has already been sent for this page.
 *
 * The instrument fires once and then costs nothing. See the guard in
 * `__clicheShow` for why that matters: the second call site is the only part of
 * this tracer that sits inside a measured step.
 */
let phaseShowEnteredReported = false;

// PHASE ONE, and its POSITION is the measurement: the first statement this
// module executes, above the element lookups below - which throw. A `loaded`
// line therefore proves the script STARTED, and says nothing about whether it
// finished; that is exactly the question hypothesis (a) asks.
//
// WHAT A MISSING `loaded` LINE DOES NOT PROVE, because the table in
// `veil::veil_ready` would otherwise be read as more than it is: it proves that
// no line ARRIVED. Either the script never ran - hypothesis (a) - or outgoing
// IPC does not leave a window that has never been shown, which would explain the
// missing `veil_decoded` just as well. The two are told apart by the run-1
// `show-entered` line: if THAT one arrives while `loaded` never did, IPC out of
// a hidden window works and it is this call, at module load, that was lost.
//
// It runs during the preheat, seconds before any shortcut, so it costs the
// measured pipeline nothing.
reportPhase('loaded');

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
const toastRegion = document.getElementById('toast-region');
const toast = document.getElementById('toast');
const toastMeasurement = document.getElementById('toast-measurement');
const toastLead = document.getElementById('toast-lead');
const toastText = document.getElementById('toast-text');
const toastDismiss = document.getElementById('toast-dismiss');

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
// The confirmation, and the failure. Same reasoning again, and the stake is the
// same one #edge carries: a capture whose outcome nothing reports is the defect
// of 5 September 2026 - the screen came back with no word either way, and a
// clipboard that had refused looked exactly like one that had not.
if (!(toastRegion instanceof HTMLElement)) {
  throw new Error('Cannot run the veil: #toast-region is missing from veil.html');
}
if (!(toast instanceof HTMLElement)) {
  throw new Error('Cannot run the veil: #toast is missing from veil.html');
}
if (!(toastMeasurement instanceof HTMLElement)) {
  throw new Error('Cannot run the veil: #toast-measurement is missing from veil.html');
}
if (!(toastLead instanceof HTMLElement)) {
  throw new Error('Cannot run the veil: #toast-lead is missing from veil.html');
}
if (!(toastText instanceof HTMLElement)) {
  throw new Error('Cannot run the veil: #toast-text is missing from veil.html');
}
// A BUTTON, not merely an element: `hidden` and `aria-label` would work on any
// tag, and a <div> would take neither the keyboard nor the focus ring the
// design system draws on `.c-btn`. This is the only control in the veil.
if (!(toastDismiss instanceof HTMLButtonElement)) {
  throw new Error('Cannot run the veil: #toast-dismiss is missing from veil.html');
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

/** The same read, for a duration token. See `millisecondsIn`. */
const tokenMilliseconds = (name: string): number => {
  const raw = getComputedStyle(root).getPropertyValue(name);

  try {
    return millisecondsIn(raw);
  } catch (error: unknown) {
    throw new Error(
      `Cannot run the veil: ${name} does not resolve to a duration (${String(error)})`,
    );
  }
};

/**
 * How long the confirmation owns the screen, read from the token layer.
 *
 * RE-READ AT EVERY CONFIRMATION, and not resolved once like the two geometry
 * tokens above. `--dur-short` falls to `0s` under `prefers-reduced-motion`, and
 * that is a setting the user can change while this application is running; a
 * value frozen at the preheat would go on describing the setting they had when
 * they launched it. Two style reads, once per successful capture, at human
 * speed and far outside the 150 ms budget.
 */
const toastTiming = (): ToastTiming => ({
  dwellMs: tokenMilliseconds('--dur-toast-dwell'),
  fadeMs: tokenMilliseconds('--dur-short'),
});

// Called once during the preheat, for its THROW and not for its value: a
// mistyped or deleted duration token then fails where every other token error
// in this file does - at load, seconds before any shortcut - rather than on the
// first capture somebody makes.
toastTiming();

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

/**
 * The timer that closes a confirmation, while one is pending.
 *
 * It carries the run it belongs to in its closure. `null` means nothing is
 * counting down - which is the state of every capture that has not yet been
 * validated, and of one whose confirmation has already fired.
 */
let confirmationTimer: number | null = null;

/** Stops a pending confirmation from closing a veil it no longer owns. */
const clearConfirmationTimer = (): void => {
  if (confirmationTimer !== null) {
    window.clearTimeout(confirmationTimer);
    confirmationTimer = null;
  }
};

/**
 * Puts the toast away. Called as soon as the user acts again, and on the way
 * out of every capture.
 *
 * The tone classes go with it, so the element is back to the neutral
 * `c-note c-toast` the document was parsed with: a toast that kept
 * `c-note--danger` would flash red for one frame the next time it is shown as a
 * success, and `c-toast--transient` left behind would start a fade nobody asked
 * for.
 */
const hideToast = (): void => {
  if (toastRegion.hidden) {
    return;
  }
  toastRegion.hidden = true;
  toast.className = 'c-note c-toast';
  toast.removeAttribute('role');
  toastMeasurement.hidden = true;
  toastMeasurement.textContent = '';
  toastLead.hidden = true;
  toastLead.textContent = '';
  toastText.textContent = '';
  toastDismiss.hidden = true;
};

/**
 * Draws a plan. Nothing here decides anything - see `./confirmation`.
 *
 * The class list is written WHOLE rather than added to, so the element cannot
 * accumulate a tone from a previous message; `hideToast` above does the same on
 * the way out, and either alone would be enough. Both are kept because the
 * failure they prevent is silent.
 */
const showToast = (plan: ToastPlan): void => {
  toast.className = plan.classes.join(' ');
  toast.setAttribute('role', plan.role);

  toastMeasurement.textContent = plan.measurement ?? '';
  toastMeasurement.hidden = plan.measurement === null;
  toastLead.textContent = plan.lead ?? '';
  toastLead.hidden = plan.lead === null;
  toastText.textContent = plan.text;
  toastDismiss.hidden = !plan.dismissable;

  toastRegion.hidden = false;
};

// The button's label, written once during the preheat. It is `aria-label` and
// not text because the control is an icon: `.c-btn--icon` is 44 px of glyph,
// and PRD A2 gives it the same focus ring as every other control.
toastDismiss.setAttribute('aria-label', UI_STRINGS.dismissMessage);

toastDismiss.addEventListener('click', (event: MouseEvent) => {
  event.preventDefault();
  hideToast();
});

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
  hideToast();
  clearConfirmationTimer();
  frame.hidden = true;
  frame.removeAttribute('src');
  // `is-confirmed` is deliberately NOT removed here. This function's only
  // caller is Escape, and Escape during a confirmation is followed by
  // `veil_dismissed`, which hides the window a round trip later: taking the
  // class off now would repaint the page opaque black for those few frames,
  // over the desktop the user can currently see. `__clicheShow` removes it
  // instead, before the next capture is drawn.
};

window.__clicheShow = (source: string, run: number): void => {
  // PHASE TWO, on the FIRST line and before `frame` is touched, because what it
  // has to establish is that this function was ENTERED at all - not that it got
  // anywhere. `loaded` present and this one missing on run 1 is hypothesis (b):
  // `window.eval` does not reach a window that has never been shown.
  //
  // ONCE PER PAGE, and that guard is the whole reason this instrument can live
  // on the critical path. This line sits inside the `decoded` step - between
  // `transport` and the mark `veil_decoded` takes - so an unconditional call
  // would make every run pay a serialisation and an IPC post, and `decoded`
  // would stop being comparable with the figure measured on a134cb7. The
  // question it answers is about the FIRST capture after launch, and that run
  // is discarded from the measurement anyway: it is the one the fallback
  // rescues. So the cost lands only where there is no measurement to spoil,
  // and runs 2 onward are byte-for-byte the path a134cb7 measured.
  if (!phaseShowEnteredReported) {
    phaseShowEnteredReported = true;
    reportPhase('show-entered');
  }

  // A rectangle, a hint or a message left over from the previous capture must
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
  hideToast();

  // THE END OF A CONFIRMATION, whichever way it ends. The timer of the previous
  // capture is dropped here - it belongs to a run that is over, and left alone
  // it would ask Rust to take down the veil this call is about to fill. Rust
  // refuses that on the run number as well (`veil_confirmed`); this is the half
  // on the side that knows a new capture has started.
  //
  // The class comes off in the same breath, and this is the ONLY place it does:
  // it runs while the window is still hidden, so the page is opaque again
  // before anything is shown. See `reset` for why Escape does not do it.
  clearConfirmationTimer();
  root.classList.remove('is-confirmed');

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

  hideToast();

  // The four numbers go over as measured. No scale, no rounding - see the file
  // header.
  void invoke('veil_selected', {
    run,
    x0: rect.left,
    y0: rect.top,
    x1: rect.right,
    y1: rect.bottom,
  })
    .then((reply: unknown) => {
      // Only if a newer capture has not started in the meantime. Same reasoning
      // as the decode acknowledgement above: a reply belonging to run 3 must
      // not tear down run 4.
      if (currentRun !== run) {
        return;
      }
      // `reply` is `unknown` on purpose, and `sizeLabel` validates it rather
      // than casting: `invoke<T>` is an assertion TypeScript cannot check about
      // a value that crossed an IPC frontier.
      enterConfirmation(
        run,
        planFor({ copied: true, size: sizeLabel(reply) }, toastTiming()),
      );
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
      // "too small" is to make it bigger, which is a gesture away. The veil
      // stays opaque and goes on answering the pointer - `AfterSelection` in
      // src-tauri/src/veil.rs is the other half of that rule.
      console.error('[cliche] veil: the selection was refused', error);
      if (currentRun !== run) {
        return;
      }
      // Rust's own sentence, shown as it arrived. It is English, in a French
      // interface, and that is the lesser of the two faults available: the
      // maquette's specimen wording names ONE cause - another application
      // holding the clipboard - and the refusal a user meets most often is a
      // selection too small to be a deliberate drag.
      showToast(
        planFor(
          {
            copied: false,
            reason: typeof error === 'string' ? error : String(error),
          },
          toastTiming(),
        ),
      );
    });
};

/**
 * The capture worked: step the veil out of the way and say so.
 *
 * THE FROZEN SCREEN GOES FIRST, and that line is what makes the word
 * "transparent" mean anything: this window shows a pixel-exact copy of the
 * desktop, so a transparent PAGE over a visible image is indistinguishable from
 * the image. The band and the page's own black background go with it, in CSS,
 * through `is-confirmed` on the root.
 *
 * The window's half of the same move - per-pixel alpha, and no longer answering
 * the pointer - is done in Rust: `create` builds it transparent, `veil_selected`
 * sets the pass-through once the clipboard has taken the image. Neither can be
 * done from here.
 *
 * `currentRun` goes to zero, which is what stops every pointer and key handler
 * in this file: there is nothing left to select, resize or copy.
 */
const enterConfirmation = (run: number, plan: ToastPlan): void => {
  corners = null;
  gesture = null;
  before = null;
  releaseCapture();
  gesturePointer = null;
  setCursor(null);
  selection.hidden = true;
  hint.hidden = true;
  frame.hidden = true;
  frame.removeAttribute('src');
  currentRun = 0;

  root.classList.add('is-confirmed');
  showToast(plan);

  if (plan.dwellMs === null) {
    // Unreachable for a copy that worked, and not an assertion: a message that
    // never leaves is a legitimate plan, and the veil would then wait for
    // Escape rather than close itself.
    return;
  }

  // A TIMER AND NOT `animationend`, deliberately. `.c-toast--transient` fades
  // the message out on its own, from the same token this delay is read from, so
  // the two agree by construction - but the window is what has to come down,
  // and an animation event that never fires would leave a transparent,
  // click-through, always-on-top window over the desktop. A timer in a visible
  // page always runs.
  //
  // The delay is the dwell PLUS the fade, both from `tokens.css`: closing at the
  // dwell alone would cut `c-toast-out` off at its first frame.
  confirmationTimer = window.setTimeout(() => {
    confirmationTimer = null;
    void invoke('veil_confirmed', { run }).catch((error: unknown) => {
      console.error('[cliche] veil: could not close the confirmation', error);
    });
  }, plan.dwellMs);
};

window.addEventListener('pointerdown', (event: PointerEvent) => {
  // THE TOAST IS NOT THE VEIL, and this is checked FIRST - before the button
  // and before `currentRun`. `.c-toast` sets `pointer-events: auto` inside a
  // region that sets `none`, so the message really does receive the press that
  // lands on it; without this line, pressing its dismiss button would also
  // start drawing a rectangle underneath, and `hideToast` below would take the
  // button out from under the finger before the `click` that follows.
  //
  // `contains` and not an identity check: the press lands on the <svg> or its
  // <path>, never on the button itself.
  if (event.target instanceof Node && toastRegion.contains(event.target)) {
    return;
  }

  // The primary button only: a right-click is not a selection. And nothing is
  // drawn when no capture is on screen, which is what `currentRun === 0` means.
  if (event.button !== 0 || currentRun === 0 || gesture !== null) {
    return;
  }

  // Stops WebView2 starting a native drag or a text selection under the hand.
  event.preventDefault();

  // The user is acting again, so the message about the last attempt has said
  // what it had to say. A failure toast is dismissable by its own button too -
  // components.css: "It leaves when it is dismissed" - and this is the second
  // way out, for the far more common gesture of simply correcting the
  // rectangle.
  hideToast();

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
