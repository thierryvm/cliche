/**
 * THE ZONE MODEL of the veil: pure, DOM-free, and under test.
 *
 * # Why this is a file of its own
 *
 * Everything here used to live at the top of `src/veil/main.ts`, above a note
 * saying it was "the only part of this file that could be unit-tested if this
 * repository had a JavaScript test runner". The runner arrived on 4 September
 * 2026 (Vitest, see `vitest.config.ts`), and the note stopped being true - but
 * the functions still could not be reached, for a structural reason and not a
 * missing dependency: `main.ts` QUERIES THE DOM AT MODULE LOAD. It reads five
 * elements, throws if any is missing, resolves two CSS tokens through
 * `getComputedStyle`, and installs pointer listeners. Importing it from a Node
 * test environment throws before the first `expect`.
 *
 * Splitting the pure half out is what makes it testable WITHOUT a simulated DOM
 * - no jsdom, no happy-dom, no dependency beyond the runner itself. `main.ts`
 * imports from here and keeps every DOM-touching line.
 *
 * # WHAT MOVED, AND WHAT DID NOT
 *
 * The move was mechanical: the bodies below are the ones that were in `main.ts`
 * on 4 September 2026, character for character. The comment above `hitTest`
 * lost the paragraph that announced these functions were untestable, because it
 * had become false, and `CursorName` gained an `export` it needed to cross a
 * file boundary. Nothing else was reworded, reordered or simplified: the
 * owner validated this behaviour on screen the day before, and a reformulation
 * taken on the way out would be a regression no existing test could catch.
 *
 * Left behind in `main.ts` on purpose, because they read the DOM or the window:
 * `rectOf` and `clamp` are pure but only ever fed by `PointerEvent`s, `at`,
 * `tokenPixels`, `setCursor` and `drawSelection` are not pure at all.
 */

/** A point in the veil document, in CSS pixels. */
export interface Point {
  readonly x: number;
  readonly y: number;
}

/** A rectangle by its four edges, already normalised. */
export interface Rect {
  readonly left: number;
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
}

/**
 * What the pointer is over: one of the eight resize zones, the interior, or
 * beyond the whole affair.
 */
export type Zone =
  | 'nw'
  | 'n'
  | 'ne'
  | 'w'
  | 'e'
  | 'sw'
  | 's'
  | 'se'
  | 'inside'
  | 'outside';

/**
 * Which zone a point falls in.
 *
 * # The model: ONE RING, ENTIRELY OUTSIDE THE RECTANGLE
 *
 * Take the rectangle grown by `outset` on all four sides, and subtract the
 * rectangle. What is left is a ring, cut by the extensions of the four edges
 * into eight cells: NW, N, NE, W, E, SW, S, SE.
 *
 * Everything follows from that one decision:
 *
 * - **No grab zone ever covers a selected pixel, at any size.** So "move it by
 *   its middle" survives down to the smallest selection this tool accepts,
 *   8 x 8 physical px (`clipboard::MIN_COPYABLE_AREA_PX`), where a ring drawn
 *   half-inside would have swallowed the interior whole.
 * - **No zone overlaps another**, by construction. There is no priority rule to
 *   write, and therefore none to get wrong: the cells partition the ring.
 * - The resolution order is a consequence, not a policy: outside the grown
 *   rectangle is `outside`, inside the rectangle is `inside`, and what remains
 *   is the ring cell the point is in.
 *
 * The rectangle's own edge belongs to `inside`. A point exactly on `left` is
 * one CSS pixel of the image the user chose, and moving is the gesture that
 * cannot destroy anything.
 *
 * # WHY `outset` IS 12 px AND NOT THE 44 px OF `--hit-min`
 *
 * Because 44 is arithmetically impossible here, not because it was inconvenient.
 * Eight disjoint zones of 44 px need a 132 x 132 px selection; the smallest
 * legal one is 8 x 8. The exception, and what makes it acceptable - missing a
 * grip costs a `move` or a `redraw`, both undone by the next gesture - is
 * written out in veil.html next to the token.
 *
 * # UNDER TEST, since 4 September 2026
 *
 * `src/veil/zones.test.ts` sweeps the ten outcomes at ordinary sizes, at 8 x 8
 * - the smallest legal selection - and on a 200 x 1 strip, and asserts the two
 * properties the model exists for: exactly one outcome per point, and no zone
 * covering an interior pixel. The anchor rule this model feeds is ALSO under
 * test in Rust, in `capture.rs`,
 * `every_grip_anchors_on_the_side_it_is_not_moving`: two independent statements
 * of one rule, in two languages, which is a real signal the day they disagree.
 */
export const hitTest = (rect: Rect, point: Point, outset: number): Zone => {
  const west = point.x < rect.left;
  const east = point.x > rect.right;
  const north = point.y < rect.top;
  const south = point.y > rect.bottom;

  if (!west && !east && !north && !south) {
    return 'inside';
  }

  if (
    point.x < rect.left - outset ||
    point.x > rect.right + outset ||
    point.y < rect.top - outset ||
    point.y > rect.bottom + outset
  ) {
    return 'outside';
  }

  if (north) {
    return west ? 'nw' : east ? 'ne' : 'n';
  }
  if (south) {
    return west ? 'sw' : east ? 'se' : 's';
  }
  return west ? 'w' : 'e';
};

/** The five cursor names veil.html knows, plus the absent one: `crosshair`. */
export type CursorName = 'move' | 'nwse' | 'nesw' | 'ns' | 'ew' | null;

/** What the pointer should look like over a zone it is merely hovering. */
export const cursorForZone = (zone: Zone): CursorName => {
  switch (zone) {
    case 'nw':
    case 'se':
      return 'nwse';
    case 'ne':
    case 'sw':
      return 'nesw';
    case 'n':
    case 's':
      return 'ns';
    case 'w':
    case 'e':
      return 'ew';
    case 'inside':
      return 'move';
    case 'outside':
      return null;
  }
};

/**
 * How a grip re-anchors the drag: which corner stays put, and which of the
 * pointer's two coordinates the moving corner takes.
 *
 * A corner grip follows both. A side grip follows one and pins the other to the
 * edge that is not moving - which is what keeps a north drag from dragging the
 * left and right edges with it.
 */
export interface Grab {
  readonly anchor: Point;
  readonly followX: boolean;
  readonly followY: boolean;
  readonly pinned: Point;
}

/**
 * Where the moving corner goes for a pointer at `point`.
 *
 * The pin is not a detail: without it, grabbing the north edge would take the
 * pointer's x as well and the rectangle would collapse to a line the instant it
 * was touched. Used at the press AND on every move, so the two cannot disagree.
 */
export const movingCorner = (grab: Grab, point: Point): Point => ({
  x: grab.followX ? point.x : grab.pinned.x,
  y: grab.followY ? point.y : grab.pinned.y,
});

/**
 * The anchor rule, in one place. Mirrored in `capture.rs`'s test model, where
 * it is under test at every one of the eight grips.
 *
 * Returns `null` for the two zones that are not a resize.
 */
export const grabFor = (zone: Zone, rect: Rect): Grab | null => {
  const both = (anchor: Point): Grab => ({
    anchor,
    followX: true,
    followY: true,
    pinned: anchor,
  });

  switch (zone) {
    case 'nw':
      return both({ x: rect.right, y: rect.bottom });
    case 'ne':
      return both({ x: rect.left, y: rect.bottom });
    case 'sw':
      return both({ x: rect.right, y: rect.top });
    case 'se':
      return both({ x: rect.left, y: rect.top });
    case 'n':
      return {
        anchor: { x: rect.left, y: rect.bottom },
        followX: false,
        followY: true,
        pinned: { x: rect.right, y: rect.bottom },
      };
    case 's':
      return {
        anchor: { x: rect.left, y: rect.top },
        followX: false,
        followY: true,
        pinned: { x: rect.right, y: rect.top },
      };
    case 'w':
      return {
        anchor: { x: rect.right, y: rect.top },
        followX: true,
        followY: false,
        pinned: { x: rect.right, y: rect.bottom },
      };
    case 'e':
      return {
        anchor: { x: rect.left, y: rect.top },
        followX: true,
        followY: false,
        pinned: { x: rect.left, y: rect.bottom },
      };
    case 'inside':
    case 'outside':
      return null;
  }
};

/**
 * Reads a length token as a number of CSS pixels.
 *
 * `rem` has to be handled: a custom property is substituted as written, so
 * `--space-3` reaches here as `0.75rem` and not as `12px`. Converting against
 * the root font size is also what makes these two lengths follow Windows text
 * scaling, like every other rem in this design system.
 *
 * Anything else THROWS, at load, during the preheat. A fallback number would be
 * the second copy of a token that tokens.css opens by forbidding, and - worse -
 * would let a mistyped token pass as a plausible 12 px that nobody would ever
 * look at again.
 */
export const lengthInPixels = (raw: string, rootFontSize: number): number => {
  const text = raw.trim();
  const value = Number.parseFloat(text);

  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`"${raw}" is not a positive length`);
  }
  if (text.endsWith('px')) {
    return value;
  }
  if (text.endsWith('rem')) {
    return value * rootFontSize;
  }

  throw new Error(`"${raw}" is a length in a unit this page cannot resolve`);
};
