/*
 * A key combination, drawn as one chip per key.
 *
 * WHY THIS IS A COPY OF THE SHOWCASE'S `Keys`, AND WHY THAT IS NOT A DRIFT
 *   `src/design/Showcase.tsx` has the same eight lines and does not export
 *   them, on purpose: that page has to open in a plain browser tab and imports
 *   nothing from the application, so the arrow can only ever point this way.
 *   What keeps the two honest is not a shared module but the material layer -
 *   `.c-keys` and `.c-kbd` are declared once in components.css, and the
 *   comments there hold the reasoning (a combination folds instead of
 *   crushing, and the widest cap in the product measures 71 px).
 *
 * NO COMBINATION IS WRITTEN HERE EITHER. The keys arrive from the caller,
 * which got them from the Rust registry - see `src/shortcut-hint.ts`.
 */

import { Fragment } from 'react';

import './design/components.css';

/** One chip per key, in a row that wraps. */
export default function Keys({ keys }: { readonly keys: readonly string[] }) {
  return (
    <span className="c-keys">
      {keys.map((key, index) => (
        <Fragment key={`${key}-${String(index)}`}>
          {index > 0 && <span aria-hidden="true">+</span>}
          <span className="c-kbd">{key}</span>
        </Fragment>
      ))}
    </span>
  );
}
