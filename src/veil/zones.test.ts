/**
 * The zone model of the veil, under test.
 *
 * Everything here is pure arithmetic on numbers: no DOM, no fixture, no
 * simulated window. That is the whole reason `zones.ts` was split out of
 * `main.ts` - see the header there.
 *
 * # THE PROPERTY THIS FILE EXISTS FOR
 *
 * `hitTest` is documented as a partition: "no zone overlaps another, by
 * construction. There is no priority rule to write, and therefore none to get
 * wrong." That claim had never been checked. It is checked here against a
 * SECOND statement of the model, written below from the geometry - three
 * vertical bands crossed with three horizontal bands, inside a grown box - and
 * not from `hitTest`'s branches. Two implementations that agree over tens of
 * thousands of points is evidence; one implementation agreeing with itself is
 * not.
 *
 * # AND THE ONE `capture.rs` ALREADY STATES IN RUST
 *
 * The grip tests below use the same rectangle and the same target points as
 * `every_grip_anchors_on_the_side_it_is_not_moving` in
 * `src-tauri/src/capture.rs`. Deliberately: the anchor rule is written twice,
 * in two languages, and the day the two disagree that is a real signal rather
 * than a coincidence to reconcile.
 */

import { describe, expect, it } from 'vitest';

import {
  cursorForZone,
  grabFor,
  hitTest,
  lengthInPixels,
  movingCorner,
} from './zones';
import type { CursorName, Point, Rect, Zone } from './zones';

const ZONES: readonly Zone[] = [
  'nw',
  'n',
  'ne',
  'w',
  'e',
  'sw',
  's',
  'se',
  'inside',
  'outside',
];

/**
 * The ten memberships, each written on its own from the geometric description
 * and never by negating the previous one.
 *
 * A band pair: `x` is west of the rectangle, within it, or east of it; `y` is
 * north, within, or south. Eight of the nine crossings are ring cells, the
 * ninth is the interior, and anything beyond the box grown by `outset` is
 * outside. Written this way, "exactly one holds" is a real assertion about the
 * model rather than a restatement of a chain of `else`s.
 */
const holds: Record<Zone, (rect: Rect, p: Point, outset: number) => boolean> = {
  inside: (r, p) => p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom,
  outside: (r, p, o) =>
    p.x < r.left - o || p.x > r.right + o || p.y < r.top - o || p.y > r.bottom + o,

  nw: (r, p, o) =>
    p.x >= r.left - o && p.x < r.left && p.y >= r.top - o && p.y < r.top,
  n: (r, p, o) =>
    p.x >= r.left && p.x <= r.right && p.y >= r.top - o && p.y < r.top,
  ne: (r, p, o) =>
    p.x > r.right && p.x <= r.right + o && p.y >= r.top - o && p.y < r.top,

  w: (r, p, o) =>
    p.x >= r.left - o && p.x < r.left && p.y >= r.top && p.y <= r.bottom,
  e: (r, p, o) =>
    p.x > r.right && p.x <= r.right + o && p.y >= r.top && p.y <= r.bottom,

  sw: (r, p, o) =>
    p.x >= r.left - o && p.x < r.left && p.y > r.bottom && p.y <= r.bottom + o,
  s: (r, p, o) =>
    p.x >= r.left && p.x <= r.right && p.y > r.bottom && p.y <= r.bottom + o,
  se: (r, p, o) =>
    p.x > r.right && p.x <= r.right + o && p.y > r.bottom && p.y <= r.bottom + o,
};

/** Which of the ten memberships hold at a point. Should always be exactly one. */
const claiming = (rect: Rect, point: Point, outset: number): Zone[] =>
  ZONES.filter((zone) => holds[zone](rect, point, outset));

/**
 * Every integer point of the grown box, plus a two-pixel margin so that the
 * `outside` verdict is reached from all four sides and not merely at a corner.
 */
const sweep = (rect: Rect, outset: number): Point[] => {
  const points: Point[] = [];
  for (let x = rect.left - outset - 2; x <= rect.right + outset + 2; x += 1) {
    for (let y = rect.top - outset - 2; y <= rect.bottom + outset + 2; y += 1) {
      points.push({ x, y });
    }
  }
  return points;
};

/**
 * The three shapes that matter, and why each is here.
 *
 * `ordinary` is a rectangle nobody would think twice about. The other two are
 * the sizes the model was designed around and the ones where a ring drawn even
 * slightly wrong stops being a partition:
 *
 * - 8 x 8 is the SMALLEST selection this tool copies at all
 *   (`clipboard::MIN_COPYABLE_AREA_PX` is 64, and 8 x 8 is 64). A ring drawn
 *   half inside the rectangle would swallow its interior whole at this size.
 * - 200 x 1 is the thin strip `clipboard.rs` explicitly allows - "a gesture
 *   somebody makes on purpose, and its area of 200 says so". One axis is a
 *   single pixel thick, so `n` and `s` are adjacent with the interior between
 *   them and nothing to spare.
 */
const SHAPES: readonly { name: string; rect: Rect }[] = [
  { name: 'ordinary 120 x 80', rect: { left: 100, top: 60, right: 220, bottom: 140 } },
  { name: 'smallest legal 8 x 8', rect: { left: 40, top: 40, right: 48, bottom: 48 } },
  { name: 'thin strip 200 x 1', rect: { left: 10, top: 300, right: 210, bottom: 301 } },
];

/**
 * `outset` is an ARGUMENT of `hitTest`, not a constant of the module: the veil
 * reads it from `--veil-grip-outset` at load. So the properties are checked at
 * three widths rather than at the token's current value - 12 is the one in use
 * on 4 September 2026, 1 is the degenerate ring, 40 is wider than the smallest
 * selection is tall, which is the case a reader would expect to break.
 */
const OUTSETS: readonly number[] = [1, 12, 40];

describe('hitTest', () => {
  it('gives every point exactly one of the ten outcomes', () => {
    const disputed: string[] = [];

    for (const shape of SHAPES) {
      for (const outset of OUTSETS) {
        for (const point of sweep(shape.rect, outset)) {
          const claims = claiming(shape.rect, point, outset);
          if (claims.length !== 1) {
            disputed.push(
              `${shape.name} outset ${outset} at (${point.x},${point.y}): ${
                claims.length === 0 ? 'no zone' : claims.join('+')
              }`,
            );
          }
        }
      }
    }

    expect(disputed).toEqual([]);
  });

  it('returns the zone the geometry says, at every swept point', () => {
    const wrong: string[] = [];

    for (const shape of SHAPES) {
      for (const outset of OUTSETS) {
        for (const point of sweep(shape.rect, outset)) {
          const expected = claiming(shape.rect, point, outset)[0];
          const actual = hitTest(shape.rect, point, outset);
          if (actual !== expected) {
            wrong.push(
              `${shape.name} outset ${outset} at (${point.x},${point.y}): ` +
                `hitTest said ${actual}, the geometry says ${String(expected)}`,
            );
          }
        }
      }
    }

    expect(wrong).toEqual([]);
  });

  it('reaches all ten outcomes on every shape, so the sweep is not vacuous', () => {
    for (const shape of SHAPES) {
      for (const outset of OUTSETS) {
        const seen = new Set<Zone>();
        for (const point of sweep(shape.rect, outset)) {
          seen.add(hitTest(shape.rect, point, outset));
        }
        expect([...seen].sort()).toEqual([...ZONES].sort());
      }
    }
  });

  it('never lets a grab zone cover a selected pixel', () => {
    // THE property the ring exists for. Every pixel the user chose - edges
    // included, the doc comment is explicit that `left` itself is interior -
    // must answer `inside`, at any outset and at any size. Got wrong at 8 x 8,
    // the whole selection would become ungrabbable by its middle.
    const covered: string[] = [];

    for (const shape of SHAPES) {
      for (const outset of OUTSETS) {
        for (let x = shape.rect.left; x <= shape.rect.right; x += 1) {
          for (let y = shape.rect.top; y <= shape.rect.bottom; y += 1) {
            const zone = hitTest(shape.rect, { x, y }, outset);
            if (zone !== 'inside') {
              covered.push(`${shape.name} outset ${outset} at (${x},${y}): ${zone}`);
            }
          }
        }
      }
    }

    expect(covered).toEqual([]);
  });

  it('puts the rectangle corners inside and the pixel diagonally beyond in the corner cell', () => {
    // The four seams, stated by hand rather than swept, because an off-by-one
    // here is the difference between "the corner resizes" and "the corner
    // moves" and it deserves to be readable.
    const rect: Rect = { left: 40, top: 40, right: 48, bottom: 48 };

    expect(hitTest(rect, { x: 40, y: 40 }, 12)).toBe('inside');
    expect(hitTest(rect, { x: 48, y: 48 }, 12)).toBe('inside');
    expect(hitTest(rect, { x: 39, y: 39 }, 12)).toBe('nw');
    expect(hitTest(rect, { x: 49, y: 39 }, 12)).toBe('ne');
    expect(hitTest(rect, { x: 39, y: 49 }, 12)).toBe('sw');
    expect(hitTest(rect, { x: 49, y: 49 }, 12)).toBe('se');

    // The last pixel of the ring, and the first one past it.
    expect(hitTest(rect, { x: 28, y: 44 }, 12)).toBe('w');
    expect(hitTest(rect, { x: 27, y: 44 }, 12)).toBe('outside');
    expect(hitTest(rect, { x: 60, y: 44 }, 12)).toBe('e');
    expect(hitTest(rect, { x: 61, y: 44 }, 12)).toBe('outside');
  });

  it('works on fractional coordinates, which is what a pointer actually reports', () => {
    // Nothing in the veil rounds a pointer coordinate - see the header of
    // main.ts. A model that only held on integers would be a model of
    // something else.
    const rect: Rect = { left: 10.5, top: 20.25, right: 30.5, bottom: 40.75 };

    expect(hitTest(rect, { x: 10.5, y: 20.25 }, 12)).toBe('inside');
    expect(hitTest(rect, { x: 10.4999, y: 20.25 }, 12)).toBe('w');
    expect(hitTest(rect, { x: 10.4999, y: 20.2499 }, 12)).toBe('nw');
    expect(hitTest(rect, { x: 30.5001, y: 40.7501 }, 12)).toBe('se');
    expect(hitTest(rect, { x: 30.5, y: 40.75 }, 12)).toBe('inside');
  });
});

describe('cursorForZone', () => {
  it('names one cursor per direction, and none outside', () => {
    const expected: Record<Zone, CursorName> = {
      nw: 'nwse',
      se: 'nwse',
      ne: 'nesw',
      sw: 'nesw',
      n: 'ns',
      s: 'ns',
      w: 'ew',
      e: 'ew',
      inside: 'move',
      outside: null,
    };

    for (const zone of ZONES) {
      expect(`${zone} -> ${String(cursorForZone(zone))}`).toBe(
        `${zone} -> ${String(expected[zone])}`,
      );
    }
  });

  it('gives opposite grips the same diagonal, which is what makes the arrow readable', () => {
    // A cursor is a claim about an AXIS, not about a direction: nw and se pull
    // along the same diagonal. If these ever differ, one of the two grips is
    // drawing an arrow that points across the drag instead of along it.
    expect(cursorForZone('nw')).toBe(cursorForZone('se'));
    expect(cursorForZone('ne')).toBe(cursorForZone('sw'));
    expect(cursorForZone('n')).toBe(cursorForZone('s'));
    expect(cursorForZone('w')).toBe(cursorForZone('e'));

    // And the two diagonals are not the same one.
    expect(cursorForZone('nw')).not.toBe(cursorForZone('ne'));
  });
});

/**
 * The four edges a grab produces, which is what the Rust model's `grab` returns
 * and what this file has to compare against.
 *
 * A local model on purpose, like `Drag` in capture.rs: reusing `main.ts`'s
 * `rectOf` would mean importing the DOM half, and a test that normalises with
 * the same code it is checking proves less.
 */
const edgesOf = (anchor: Point, pointer: Point): Rect => ({
  left: Math.min(anchor.x, pointer.x),
  top: Math.min(anchor.y, pointer.y),
  right: Math.max(anchor.x, pointer.x),
  bottom: Math.max(anchor.y, pointer.y),
});

describe('grabFor and movingCorner', () => {
  /** The same start rectangle as `every_grip_anchors_...` in capture.rs. */
  const start: Rect = { left: 10, top: 20, right: 30, bottom: 40 };

  const drag = (zone: Zone, to: Point): Rect => {
    const grab = grabFor(zone, start);
    if (grab === null) {
      throw new Error(`${zone} is not a resize, so this test cannot drag it`);
    }
    return edgesOf(grab.anchor, movingCorner(grab, to));
  };

  it('anchors every corner grip on the opposite corner', () => {
    // Byte for byte the four assertions of the Rust test, so a divergence is
    // impossible to read as anything but a divergence.
    expect(grabFor('se', start)?.anchor).toEqual({ x: 10, y: 20 });
    expect(grabFor('nw', start)?.anchor).toEqual({ x: 30, y: 40 });
    expect(grabFor('ne', start)?.anchor).toEqual({ x: 10, y: 40 });
    expect(grabFor('sw', start)?.anchor).toEqual({ x: 30, y: 20 });
  });

  it('lets a side grip move one edge and leaves the other axis untouched', () => {
    // The absurd coordinate - 999 - is the Rust test's, and it is the point:
    // it proves the ignored axis really is ignored rather than merely
    // plausible.
    expect(drag('n', { x: 999, y: 12 })).toEqual({
      left: 10,
      right: 30,
      top: 12,
      bottom: 40,
    });
    expect(drag('s', { x: 999, y: 50 })).toEqual({
      left: 10,
      right: 30,
      top: 20,
      bottom: 50,
    });
    expect(drag('w', { x: 4, y: 999 })).toEqual({
      left: 4,
      right: 30,
      top: 20,
      bottom: 40,
    });
    expect(drag('e', { x: 44, y: 999 })).toEqual({
      left: 10,
      right: 44,
      top: 20,
      bottom: 40,
    });
  });

  it('keeps the anchor on the side that is NOT moving, for all eight grips', () => {
    // The rule in one sentence, checked as a rule rather than as eight
    // hand-written numbers: whatever the pointer does, the edges the grip does
    // not name stay exactly where they were.
    const untouched: Record<string, (r: Rect) => number[]> = {
      nw: (r) => [r.right, r.bottom],
      ne: (r) => [r.left, r.bottom],
      sw: (r) => [r.right, r.top],
      se: (r) => [r.left, r.top],
      n: (r) => [r.left, r.right, r.bottom],
      s: (r) => [r.left, r.right, r.top],
      w: (r) => [r.top, r.bottom, r.right],
      e: (r) => [r.top, r.bottom, r.left],
    };

    // A pointer that moves both axes by a lot, in a direction that does not
    // invert the rectangle - inversion is normalised and would blur the check.
    for (const zone of ['nw', 'n', 'ne', 'w', 'e', 'sw', 's', 'se'] as const) {
      const keep = untouched[zone];
      if (keep === undefined) {
        throw new Error(`no expectation written for ${zone}`);
      }
      const after = drag(zone, { x: 15, y: 25 });
      expect(`${zone}: ${keep(after).join(',')}`).toBe(
        `${zone}: ${keep(start).join(',')}`,
      );
    }
  });

  it('refuses to grab the two zones that are not a resize', () => {
    expect(grabFor('inside', start)).toBeNull();
    expect(grabFor('outside', start)).toBeNull();
  });

  it('pins the coordinate it does not follow, so a side drag cannot collapse the rectangle', () => {
    const north = grabFor('n', start);
    expect(north).not.toBeNull();
    if (north === null) {
      return;
    }

    expect(north.followX).toBe(false);
    expect(north.followY).toBe(true);
    // Whatever x the pointer reports, the moving corner takes the pinned one.
    expect(movingCorner(north, { x: -1000, y: 5 })).toEqual({ x: 30, y: 5 });
    expect(movingCorner(north, { x: 1000, y: 5 })).toEqual({ x: 30, y: 5 });
  });

  it('follows both axes on a corner grip', () => {
    const se = grabFor('se', start);
    expect(se).not.toBeNull();
    if (se === null) {
      return;
    }

    expect(movingCorner(se, { x: 77, y: 88 })).toEqual({ x: 77, y: 88 });
  });
});

describe('lengthInPixels', () => {
  it('reads px as itself', () => {
    expect(lengthInPixels('12px', 16)).toBe(12);
    expect(lengthInPixels('  12px  ', 16)).toBe(12);
    expect(lengthInPixels('0.5px', 16)).toBe(0.5);
  });

  it('converts rem against the root font size, which is what follows text scaling', () => {
    expect(lengthInPixels('0.75rem', 16)).toBe(12);
    // Windows text scaling at 125 % moves the root font size, and the grip ring
    // has to move with it.
    expect(lengthInPixels('0.75rem', 20)).toBe(15);
  });

  it('throws rather than falling back, on a unit it cannot resolve', () => {
    expect(() => lengthInPixels('12', 16)).toThrow(/cannot resolve/);
    expect(() => lengthInPixels('1em', 16)).toThrow(/cannot resolve/);
    expect(() => lengthInPixels('50%', 16)).toThrow(/cannot resolve/);
  });

  it('throws on anything that is not a positive length', () => {
    // A token that resolved to nothing used to be the interesting case: an
    // empty string parses to NaN, and a NaN outset would make every hit test
    // answer `inside`.
    expect(() => lengthInPixels('', 16)).toThrow(/not a positive length/);
    expect(() => lengthInPixels('   ', 16)).toThrow(/not a positive length/);
    expect(() => lengthInPixels('0px', 16)).toThrow(/not a positive length/);
    expect(() => lengthInPixels('-4px', 16)).toThrow(/not a positive length/);
    expect(() => lengthInPixels('auto', 16)).toThrow(/not a positive length/);
  });

  it('names the offending text in the message, so a mistyped token can be found', () => {
    expect(() => lengthInPixels('12vh', 16)).toThrow('"12vh"');
  });
});
