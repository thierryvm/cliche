/*
 * Cliché — the icon set. ONE list of paths, for the whole product.
 *
 * WHY THIS FILE EXISTS AT ALL
 *   The other half of an icon has always lived in the material layer:
 *   `.c-btn__glyph` sizes it, `.c-note .c-btn__glyph` aligns it against the
 *   first line of a message. Half of a component in src/design/ and the other
 *   half inside one page is why the home screen could render an error state
 *   with no glyph — the markup simply was not reachable from there.
 *
 * THE RULE THIS FILE ENFORCES
 *   A `d` string is never copied. Two icon sets start identical and stop being
 *   so at the first correction, and nothing in the build would notice. Anything
 *   that draws an icon imports from here.
 *
 * NO CSS OF ITS OWN
 *   components.css is imported here rather than assumed: a page that renders a
 *   Glyph gets the rule that sizes it, without having to know that it must.
 */

import './components.css';

type GlyphProps = { readonly d: string; readonly label?: string };

/** All icons share one 24-unit box and are stroked in currentColor. */
export function Glyph({ d, label }: GlyphProps) {
  return (
    <svg
      className="c-btn__glyph"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden={label === undefined}
      role={label === undefined ? undefined : 'img'}
      aria-label={label}
    >
      <path d={d} />
    </svg>
  );
}

export const ICON = {
  capture: 'M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3',
  pen: 'M4 20h4L19 9a2 2 0 0 0-3-3L5 17v3ZM14 7l3 3',
  mask: 'M4 6h7v5H4zM13 13h7v5h-7z',
  copy: 'M9 9h10v10H9zM5 15H4V4h11v1',
  trash: 'M4 7h16M10 7V5h4v2M6 7l1 13h10l1-13M10 11v5M14 11v5',
  check: 'M5 13l4 4L19 7',
  alert: 'M12 8v5M12 17h.01M12 3l9 17H3l9-17Z',
  info: 'M12 11v6M12 7h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z',

  /* The window controls, added 5 September 2026 with the custom title bar.
     Deliberately the plainest possible geometry: a caption button is read by
     muscle memory, and anything characterful there reads as the wrong button.
     `restore` is two overlapping squares because that is what Windows draws
     when a window is maximised — it is not decoration, it is the state. */
  minimize: 'M5 12h14',
  maximize: 'M5 5h14v14H5z',
  restore: 'M9 4h11v11h-4M4 9h11v11H4z',
  close: 'M6 6l12 12M18 6L6 18',

  /* The three capture targets. These are not bullets in front of a title: the
     three launcher actions differ ONLY by what they aim at, so the target is
     the information the tile carries first. `capture` above already draws the
     corner brackets of a zone. */
  window: 'M4 5h16v14H4zM4 9.5h16',
  screen: 'M3 5h18v11H3zM9 20h6M12 16v4',

  /* Direction, not ornament: it points where the control goes. */
  back: 'M14 6l-6 6 6 6',
} as const;
