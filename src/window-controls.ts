/**
 * The decisions the custom title bar makes, kept out of the component.
 *
 * # Why this file exists at all
 *
 * `TitleBar.tsx` talks to a real window through `@tauri-apps/api/window`, and
 * nothing in this repository can stand a real window up: there is no jsdom and
 * no testing-library, and neither is being bought to assert on markup that can
 * be looked at directly at `#/systeme`. So the split is deliberate and it is
 * the design, not a workaround - whatever is worth a test is a function of its
 * arguments and lives here; the component renders what it is handed.
 */

import type { StringKey } from './strings';

/**
 * What the second window control should say and draw right now.
 *
 * `labelKey` and `icon` are names rather than a sentence and a path: the
 * sentence lives in `src/strings.ts` and the path in `src/design/Glyph.tsx`,
 * and a module that imported either would drag React and a stylesheet into a
 * `node` test run for nothing.
 *
 * `Extract<StringKey, …>` is not decoration. It resolves to the two literals
 * when the catalogue holds them and to a NARROWER union when it does not, so
 * the day a label is renamed in `src/strings.ts` this file stops compiling
 * instead of pointing at a key that no longer exists.
 */
export interface MaximiseControl {
  readonly labelKey: Extract<StringKey, 'windowMaximize' | 'windowRestore'>;
  readonly icon: 'maximize' | 'restore';
}

/**
 * The button that toggles between maximised and restored.
 *
 * Takes the state of the window rather than reading it, because reading it is
 * an IPC round trip - see `TitleBar.tsx`, which listens for resizes so that
 * this argument is the truth and not the last guess.
 */
export function maximiseControl(maximised: boolean): MaximiseControl {
  return maximised
    ? { labelKey: 'windowRestore', icon: 'restore' }
    : { labelKey: 'windowMaximize', icon: 'maximize' };
}
