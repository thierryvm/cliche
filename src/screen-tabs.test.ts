/**
 * The three title-bar tabs, under test.
 *
 * # The defect these exist for
 *
 * Until 7 September 2026 the title bar carried two icon-only TOGGLES: press ⓘ
 * to open the help, press the same ⓘ again to come back. Nothing on the help
 * screen said so, and Thierry did not find the way back - he double-clicked the
 * bar. A screenshot of the help screen has to show the way to the capture, and
 * a pressed icon does not.
 *
 * # Why a pure function, and not a rendered component
 *
 * Same rule `window-controls.test.ts` states and for the same reason: this
 * repository has no jsdom and buys none. Whatever is worth a test is a function
 * of its arguments; `TitleBar.tsx` renders what this module returns and decides
 * nothing.
 *
 * WHAT THIS CANNOT REACH, so the green is not read as more than it is: that the
 * markup actually carries `aria-pressed`, that the labels are legible at 480 px,
 * and that a click changes the hash. The first two are looked at in the showcase
 * and in the window; the third is one assignment in `App.tsx`.
 */

import { describe, expect, it } from 'vitest';

import { HELP_ROUTE, SETTINGS_ROUTE, screenFor, screenTabs } from './screen-tabs';
import { UI_STRINGS } from './strings';

/** Every hash the window can be in, plus the two nobody plans for. */
const EVERY_HASH = ['', '#', HELP_ROUTE, SETTINGS_ROUTE, '#/inconnu', '#/aide/trop/loin'];

describe('screenFor', () => {
  it('shows the launcher when there is no fragment at all', () => {
    // Both spellings arrive in practice: `location.hash` is '' on a fresh
    // window, and clearing a fragment can leave a bare '#' behind.
    expect(screenFor('')).toBe('capture');
    expect(screenFor('#')).toBe('capture');
  });

  it('gives each of the two other screens its own hash', () => {
    expect(screenFor(HELP_ROUTE)).toBe('help');
    expect(screenFor(SETTINGS_ROUTE)).toBe('settings');
  });

  it('falls back to the launcher on a hash nobody wrote', () => {
    // A window with no address bar cannot be typed into, but a stale link in a
    // reload can. Anything unrecognised is the home screen, never a blank body.
    expect(screenFor('#/inconnu')).toBe('capture');
  });
});

describe('screenTabs', () => {
  it('draws the same three tabs, in the same order, whatever the hash', () => {
    for (const hash of EVERY_HASH) {
      expect(screenTabs(hash).map((tab) => tab.screen)).toEqual([
        'capture',
        'help',
        'settings',
      ]);
    }
  });

  it('presses exactly one tab, always', () => {
    // The whole point of the shape: a tab set where nothing is pressed says the
    // window is nowhere, and one where two are pressed says it is in two places.
    for (const hash of EVERY_HASH) {
      expect(screenTabs(hash).filter((tab) => tab.pressed)).toHaveLength(1);
    }
  });

  it('presses the tab of the screen that hash actually shows', () => {
    // `App.tsx` reads `screenFor` for the body and `screenTabs` for the bar. If
    // these two disagreed, the bar would point at a screen nobody is on.
    for (const hash of EVERY_HASH) {
      const pressed = screenTabs(hash).find((tab) => tab.pressed);
      expect(pressed?.screen).toBe(screenFor(hash));
    }
  });

  it('sends each tab to the hash that presses it back', () => {
    // The round trip is what makes the way BACK exist: pressing « Capturer »
    // from the help must land on a launcher whose « Capturer » is pressed.
    for (const tab of screenTabs('')) {
      const arrived = screenTabs(tab.hash).find((one) => one.pressed);
      expect(arrived?.screen).toBe(tab.screen);
    }
  });

  it('names each tab with a catalogued label, and never twice the same', () => {
    // The criterion is a screenshot that explains itself, so the labels are
    // words and not glyphs - and they are the catalogue's words, which is what
    // stops a fourth spelling of « Réglages » appearing in the title bar.
    const labels = screenTabs('').map((tab) => UI_STRINGS[tab.labelKey]);

    expect(labels).toEqual([UI_STRINGS.capture, UI_STRINGS.helpTitle, UI_STRINGS.settingsTitle]);
    expect(new Set(labels).size).toBe(3);
  });

  it('keeps every hash ASCII', () => {
    // A fragment is part of a URL. « #/réglages » is liable to come back from
    // `location.hash` percent-encoded while the literal it is compared against
    // is not - the comparison would then fail for a reason invisible in the
    // source. Avoided rather than measured, which is why it is asserted here.
    for (const tab of screenTabs('')) {
      expect(tab.hash).toMatch(/^[\x20-\x7e]*$/);
    }
  });
});
