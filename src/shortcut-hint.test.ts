/**
 * The launcher's shortcut reminder, under test.
 *
 * Same rule as `window-controls.test.ts`: no jsdom, no testing-library, so the
 * decision is a function of what the IPC calls gave back and the component only
 * renders its answer.
 *
 * WHAT THIS CANNOT REACH: that `describe_shortcuts` and `describe_shortcut_status`
 * are actually called, and that their outcomes are wired to the inputs below.
 * That is `Launcher.tsx`, it needs the Tauri webview, and nothing here observes
 * it.
 */

import { describe, expect, it } from 'vitest';

import { hintFor } from './shortcut-hint';
import type { RegistryRead } from './shortcut-hint';
import type { ShortcutEntry, ShortcutStatus } from './shortcuts';

/** The combination the registry holds today, in the plugin's syntax. */
const ACCELERATOR = 'Ctrl+Shift+Digit2';

/**
 * An entry shaped exactly as `describe_shortcuts` serialises one.
 *
 * The base is annotated rather than left to inference: without it the literal
 * `'capture'` would widen to `string` inside the spread and stop being a
 * `ShortcutCategory`.
 */
function entry(overrides: Partial<ShortcutEntry> = {}): ShortcutEntry {
  const base: ShortcutEntry = {
    id: 'capture-region',
    accelerator: ACCELERATOR,
    keys: ['Ctrl', 'Maj', '2'],
    descriptionKey: 'captureRegion',
    category: 'capture',
  };

  return { ...base, ...overrides };
}

/** The status `describe_shortcut_status` returns on a machine that said yes. */
const ACCEPTED: ShortcutStatus = { status: 'accepted', accelerator: ACCELERATOR };

/** The status it returns when another program is holding the combination. */
const REFUSED: ShortcutStatus = {
  status: 'refused-by-system',
  accelerator: ACCELERATOR,
  reason: 'HotKey already registered',
};

/** A read that came back, with whatever status is being put to the test. */
function read(registration: ShortcutStatus, entries: readonly ShortcutEntry[] = [entry()]): RegistryRead {
  return { status: 'read', entries, registration };
}

describe('hintFor', () => {
  it('waits while the backend is being read', () => {
    expect(hintFor({ status: 'reading' })).toEqual({ state: 'loading' });
  });

  it('shows the keys the registry states, and never a combination of its own', () => {
    const entries = [entry({ accelerator: 'Ctrl+Alt+F9', keys: ['Ctrl', 'Alt', 'F9'] })];
    const accepted: ShortcutStatus = { status: 'accepted', accelerator: 'Ctrl+Alt+F9' };

    expect(hintFor(read(accepted, entries))).toEqual({
      state: 'ready',
      keys: ['Ctrl', 'Alt', 'F9'],
    });
  });

  it('finds the capture entry by what it describes, not by where it sits', () => {
    // A registry that grows a second entry must not make the launcher announce
    // the wrong combination. `[0]` would pass every other test in this file.
    const entries = [
      entry({ id: 'dismiss-veil', accelerator: 'Escape', keys: ['Échap'], descriptionKey: 'dismissVeil' }),
      entry(),
    ];

    expect(hintFor(read(ACCEPTED, entries))).toEqual({ state: 'ready', keys: ['Ctrl', 'Maj', '2'] });
  });

  it('says the combination was REFUSED when that is what the system answered', () => {
    // THE case this lot exists for. Until 6 September 2026 this same machine
    // state drew « le registre des raccourcis n'a pas pu être lu » - a sentence
    // about a table that had been read perfectly well.
    expect(hintFor(read(REFUSED))).toEqual({ state: 'refused', keys: ['Ctrl', 'Maj', '2'] });
  });

  it('carries the keys with the refusal, because the note has to name the combination', () => {
    // PRD R4. A user cannot free a combination nobody tells them about, and a
    // refusal note with no keys in it is the one that reads as a shrug.
    const hint = hintFor(read(REFUSED));

    expect(hint.state === 'refused' && hint.keys.length).toBe(3);
  });

  it('does not call a shortcut refused when nothing ever refused it', () => {
    // `not-attempted` is ours - an unreadable combination, a plugin that failed
    // to load. Drawing the refusal note here would send the user hunting for
    // another program that is not holding anything.
    const registration: ShortcutStatus = {
      status: 'not-attempted',
      reason: 'the global-shortcut plugin failed to load',
    };

    expect(hintFor(read(registration))).toEqual({ state: 'unavailable' });
  });

  it('refuses to name a combination the status did not answer about', () => {
    // The table describes one combination and the system answered about
    // another. Drawing the table's keys under the refusal would name the wrong
    // combination to somebody trying to free it.
    const entries = [entry({ accelerator: 'Ctrl+Alt+F9', keys: ['Ctrl', 'Alt', 'F9'] })];

    expect(hintFor(read(REFUSED, entries))).toEqual({ state: 'unavailable' });
  });

  it('refuses an entry that carries no key at all', () => {
    // An empty list would draw a reminder with no keycap in it: a sentence
    // claiming a shortcut exists, with nothing to press. Worse than no line.
    expect(hintFor(read(REFUSED, [entry({ keys: [] })]))).toEqual({ state: 'unavailable' });
  });

  it('never claims there is no shortcut when the system accepted one', () => {
    // The registry describes no region capture, but Windows took the
    // combination: the shortcut WORKS and this screen simply cannot name it.
    // « Cliché tourne sans son raccourci » would be the one outright false
    // thing this module could say.
    const entries = [entry({ id: 'dismiss-veil', descriptionKey: 'dismissVeil' })];

    expect(hintFor(read(ACCEPTED, entries))).toEqual({ state: 'unreadable' });
  });

  it('says so when the backend could not be read', () => {
    expect(hintFor({ status: 'unreadable' })).toEqual({ state: 'unreadable' });
  });
});
