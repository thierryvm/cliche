/**
 * The three screens this window has, and the tabs that name them.
 *
 * # Why this is a module and not three lines inside the title bar
 *
 * Same rule `window-controls.ts` is written under, and it is the design rather
 * than a workaround: this repository has no jsdom and buys none, so whatever is
 * worth a test is a function of its arguments and lives here, while the
 * component renders what it is handed. What is worth a test here is the
 * invariant the shape depends on - EXACTLY ONE tab pressed, whatever the hash,
 * including the hashes nobody planned for.
 *
 * # The hash IS the state
 *
 * No router: three destinations. `App.tsx` writes `window.location.hash` and
 * reads it back, so the browser's own Back button keeps working and a reload
 * lands on the screen that was open. This module holds the two route literals
 * because the tab that SENDS you somewhere and the switch that decides what is
 * drawn there must not be able to disagree - they were two constants in two
 * halves of `App.tsx` until 7 September 2026.
 *
 * # ASCII in the hash, deliberately
 *
 * A fragment is part of a URL, and a non-ASCII one is liable to come back from
 * `location.hash` percent-encoded while the literal it is compared against is
 * not. « reglages » costs nothing and settles the question. Avoided rather than
 * measured, and asserted in `screen-tabs.test.ts` so the next hash cannot
 * quietly reopen it.
 */

import type { StringKey } from './strings';

/**
 * The help screen: the shortcut registry, and the monitor read-out under it.
 *
 * Reached from the title bar and from nowhere else - the tab there is what
 * makes this route exist for somebody who is not reading this file.
 */
export const HELP_ROUTE = '#/aide';

/** The settings screen: the capture combination, and the control that changes it. */
export const SETTINGS_ROUTE = '#/reglages';

/** Which of the three screens the window is showing. */
export type ScreenKey = 'capture' | 'help' | 'settings';

/**
 * One tab of the title bar.
 *
 * `labelKey` is a key and not a sentence, for the reason `MaximiseControl`
 * gives: the sentence lives in `src/strings.ts`. `Extract<StringKey, …>` is not
 * decoration either - it resolves to the three literals while the catalogue
 * holds them and to a NARROWER union when it does not, so renaming a label
 * there stops this file compiling instead of leaving a key that names nothing.
 */
export interface ScreenTab {
  readonly screen: ScreenKey;
  readonly labelKey: Extract<StringKey, 'capture' | 'helpTitle' | 'settingsTitle'>;
  /** What to write into `location.hash` to come here. */
  readonly hash: string;
  /** Drawn as `aria-pressed`. True on exactly one tab at any moment. */
  readonly pressed: boolean;
}

/**
 * The three, in the order they are drawn: what the window does most often
 * first, and the two screens ABOUT the window after it.
 *
 * The launcher's hash is the empty one on purpose - it is the screen you get
 * when nothing is asked for, so it cannot be a fragment somebody has to know.
 */
const TABS: readonly Omit<ScreenTab, 'pressed'>[] = [
  { screen: 'capture', labelKey: 'capture', hash: '' },
  { screen: 'help', labelKey: 'helpTitle', hash: HELP_ROUTE },
  { screen: 'settings', labelKey: 'settingsTitle', hash: SETTINGS_ROUTE },
];

/**
 * Which screen a hash asks for.
 *
 * Anything unrecognised is the launcher, including '' and a bare '#' - both of
 * which `location.hash` really produces. A window with no address bar cannot be
 * typed into, but a stale fragment survives a reload, and the answer to one has
 * to be a screen rather than an empty body.
 */
export function screenFor(hash: string): ScreenKey {
  // The launcher is deliberately not matched by equality: it is the fallback,
  // and giving it two spellings to recognise would be giving it two ways to be
  // wrong.
  return TABS.find((tab) => tab.hash !== '' && tab.hash === hash)?.screen ?? 'capture';
}

/**
 * The three tabs, with the one that is pressed marked.
 *
 * Exactly one, by construction: `pressed` is computed from a single answer
 * rather than from three independent comparisons, which is what makes "two tabs
 * pressed" and "none pressed" states this window cannot reach.
 */
export function screenTabs(hash: string): readonly ScreenTab[] {
  const current = screenFor(hash);

  return TABS.map((tab) => ({ ...tab, pressed: tab.screen === current }));
}
